//! macOS Calendar is the only account source. No OAuth or provider tokens.
use crate::CalendarPaths;
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

static STATE_FILE: Mutex<()> = Mutex::new(());
static INSTALL: Mutex<()> = Mutex::new(());
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
static PENDING: OnceLock<Mutex<bool>> = OnceLock::new();
fn pending() -> &'static Mutex<bool> {
    PENDING.get_or_init(|| Mutex::new(false))
}
pub fn private_json(path: &std::path::Path, value: &Value) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::create_dir_all(path.parent().ok_or("Pasta inválida")?).map_err(|e| e.to_string())?;
    let _write = STATE_FILE.lock().unwrap();
    let temp = path.with_extension("pending");
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    f.write_all(serde_json::to_string(value).unwrap().as_bytes())
        .map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}
pub fn selection(paths: &CalendarPaths) -> Vec<String> {
    fs::read(paths.selection_json())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
#[cfg(target_os = "macos")]
fn helper(paths: &CalendarPaths) -> Result<std::path::PathBuf, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let _install = INSTALL.lock().unwrap();
    let bundle = paths.root.join("bin/Tessera Calendar.app");
    let path = bundle.join("Contents/MacOS/TesseraCalendar");
    const BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/TesseraCalendar"));
    fs::create_dir_all(path.parent().unwrap())
        .map_err(|_| "Não foi possível preparar o leitor de Calendários")?;
    use sha2::{Digest, Sha256};
    let build_id = format!("{:x}", Sha256::digest(BYTES));
    let stamp = bundle.join("Contents/tessera-build-id");
    if !path.is_file() || fs::read_to_string(&stamp).ok().as_deref() != Some(build_id.as_str()) {
        let tmp = path.with_extension("new");
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o755)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        f.write_all(BYTES).map_err(|e| e.to_string())?;
        fs::rename(tmp, &path).map_err(|e| e.to_string())?;
        fs::write(
            bundle.join("Contents/Info.plist"),
            include_bytes!("../native/Info.plist"),
        )
        .map_err(|e| e.to_string())?;
        fs::write(&stamp, &build_id).map_err(|e| e.to_string())?;
        if !Command::new("/usr/bin/codesign")
            .args([
                "--force",
                "--sign",
                "-",
                "--identifier",
                "dev.tessera.calendar",
            ])
            .arg(&bundle)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Err("Não foi possível preparar a permissão de Calendários".into());
        }
    }
    Ok(path)
}
pub fn run(paths: &CalendarPaths, command: &str) -> Result<Value, String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (paths, command);
        return Err("Calendários requer macOS".into());
    }
    #[cfg(target_os = "macos")]
    {
        let exe = helper(paths)?;
        // Every read must use Launch Services, just like the permission request.
        // A direct child inherits the daemon's TCC responsibility and can report
        // not_determined even though this helper app already has full access.
        let output = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        let mut child = Command::new("/usr/bin/open")
            .args(["-W", "-n", "-g", "--stdout"])
            .arg(output.path())
            .arg(exe.parent().unwrap().parent().unwrap().parent().unwrap())
            .args(["--args", command])
            .arg(serde_json::to_string(&selection(paths)).unwrap())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "Não foi possível abrir Calendários")?;
        let timeout = if command == "connect" { 180 } else { 8 };
        let deadline = Instant::now() + Duration::from_secs(timeout);
        loop {
            // open -W can return before a short-lived command-line app finishes
            // (GetProcessPID race). The complete JSON response is the completion
            // signal, not the launcher's exit code or inherited stdout pipe.
            if let Ok(bytes) = fs::read(output.path()) {
                if let Ok(state) = serde_json::from_slice::<Value>(&bytes) {
                    if state["permission"].is_string() && state["calendars"].is_array() {
                        // Reap the launcher even if it is still waiting on macOS.
                        if matches!(child.try_wait(), Ok(None)) {
                            let _ = child.kill();
                        }
                        let _ = child.wait();
                        return Ok(state);
                    }
                }
            }
            let _ = child.try_wait();
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Calendários demoraram a responder. Tente novamente.".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
pub fn status(paths: &CalendarPaths) -> Value {
    let mut state = run(paths, "status")
        .unwrap_or_else(|e| json!({"permission":"unavailable", "message":e, "calendars":[]}));
    if state["permission"] != "authorized" {
        if let Some(error) = LAST_ERROR.lock().unwrap().as_ref() {
            state["message"] = json!(error);
        }
    }
    state["pending"] = json!(*pending().lock().unwrap());
    state["configured"] = json!(true);
    if state["message"].is_null() {
        state["message"] = json!(match state["permission"].as_str().unwrap_or("") {
            "authorized" => "Escolha os calendários que deseja mostrar. A seleção é salva automaticamente.",
            "denied" | "restricted" => "Permita Calendários para Tessera Calendar em Ajustes do Sistema → Privacidade e Segurança → Calendários.",
            _ => "Permita a leitura dos calendários já adicionados ao Mac. Nenhuma conta ou senha precisa ser importada."
        });
    }
    state
}
pub fn start(paths: CalendarPaths) -> Result<Value, String> {
    {
        let mut p = pending().lock().unwrap();
        if !*p {
            *p = true;
            *LAST_ERROR.lock().unwrap() = None;
            let paths = paths.clone();
            std::thread::spawn(move || {
                if let Err(error) = run(&paths, "connect") {
                    *LAST_ERROR.lock().unwrap() = Some(error);
                }
                *pending().lock().unwrap() = false;
            });
        }
    }
    Ok(
        json!({"pending":true,"configured":true,"message":"Confirme a permissão de Calendários na janela do macOS. Depois escolha os calendários.","calendars":[]}),
    )
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    /// Read-only check using access already granted by the human on this Mac.
    #[test]
    #[ignore = "requires human-granted Calendar access on this Mac"]
    fn installed_calendar_reads_keep_authorized_identity() {
        let paths = CalendarPaths {
            root: std::path::PathBuf::from(std::env::var("HOME").unwrap())
                .join(".tessera/calendar"),
        };
        for command in ["status", "status", "snapshot"] {
            let state = run(&paths, command).expect("native reader should respond");
            assert_eq!(state["permission"], "authorized");
            assert!(state["calendars"].as_array().is_some_and(|c| !c.is_empty()));
        }
    }
}
