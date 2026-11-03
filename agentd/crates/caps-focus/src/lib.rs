//! Read-only macOS Focus state. Mode changes belong to Control Center.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tessera_protocol::focus_line;

pub const FOCUS_PUSH_INTERVAL_S: u64 = 10;
/// Safety-net poll; Focus wakes come from DNC + Assertions mtime on mac_events.
pub const FOCUS_POLL_INTERVAL_S: u64 = 8;

const ID_MAX: usize = 63;
const NAME_MAX: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusMode {
    pub id: String,
    pub name: String,
}

pub fn encode_focus_line(
    ok: bool,
    active_id: &str,
    active_name: &str,
    modes: &[FocusMode],
) -> String {
    let pairs: Vec<(&str, &str)> = modes
        .iter()
        .map(|m| (m.id.as_str(), m.name.as_str()))
        .collect();
    focus_line(ok, active_id, active_name, &pairs)
}

pub fn focus_line_changed(last_pushed: Option<&str>, encoded: &str) -> bool {
    last_pushed != Some(encoded)
}

fn mode_db_path() -> PathBuf {
    dirs_home().join("Library/DoNotDisturb/DB/ModeConfigurations.json")
}

fn assertions_db_path() -> PathBuf {
    dirs_home().join("Library/DoNotDisturb/DB/Assertions.json")
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn sanitize(text: &str, limit: usize) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| match c {
            '\t' | '\n' | '\r' => ' ',
            c => c,
        })
        .collect();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.chars().count() > limit {
        trimmed.chars().take(limit).collect()
    } else {
        trimmed
    }
}

fn slug(name: &str) -> String {
    let folded = name.trim().to_lowercase();
    let base: String = folded
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let collapsed = base
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let s = if collapsed.is_empty() {
        "focus".to_string()
    } else {
        collapsed
    };
    s.chars().take(ID_MAX).collect()
}

fn friendly_from_id(mode_id: &str) -> Option<&'static str> {
    let low = mode_id.to_lowercase();
    const NEEDLES: &[(&str, &str)] = &[
        ("donotdisturb.mode.default", "Não Perturbe"),
        ("reduce-interruptions", "Menos Interrupções"),
        ("sleep-mode", "Sono"),
        ("personal", "Pessoal"),
        ("workout", "Exercício"),
        ("focus.work", "Trabalho"),
        ("mode.work", "Trabalho"),
        ("books", "Estudos"),
        ("driving", "Dirigindo"),
        ("gaming", "Jogos"),
        ("mindfulness", "Mindfulness"),
    ];
    for (needle, label) in NEEDLES {
        if *needle == ".work" && low.contains("workout") {
            continue;
        }
        if low.contains(needle) {
            return Some(label);
        }
    }
    None
}

fn looks_like_bundle_id(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("com.") || t.starts_with("net.") || t.contains('.')
}

fn display_name(mode_id: &str, name: &str) -> String {
    if !name.is_empty() && !looks_like_bundle_id(name) && name != mode_id {
        return sanitize(name, NAME_MAX);
    }
    if let Some(mapped) = friendly_from_id(mode_id) {
        return mapped.to_string();
    }
    if !name.is_empty() && !looks_like_bundle_id(name) {
        return sanitize(name, NAME_MAX);
    }
    "Focus".to_string()
}

fn load_plist_json(path: &Path) -> Option<Value> {
    if !path.is_file() {
        return None;
    }
    let out = Command::new("plutil")
        .args(["-convert", "json", "-o", "-", path.to_str()?])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

fn push_mode(
    out: &mut Vec<FocusMode>,
    seen: &mut std::collections::HashSet<String>,
    mode_id: &str,
    name: &str,
) {
    let id_src = if mode_id.is_empty() {
        slug(name)
    } else {
        mode_id.to_string()
    };
    let mid = sanitize(&id_src, ID_MAX);
    if mid.is_empty() || seen.contains(&mid) {
        return;
    }
    let nm = display_name(&mid, name);
    seen.insert(mid.clone());
    out.push(FocusMode { id: mid, name: nm });
}

fn modes_from_db() -> Vec<FocusMode> {
    let Some(root) = load_plist_json(&mode_db_path()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(data) = root.get("data").and_then(|d| d.as_array()) {
        for block in data {
            let configs = block.get("modeConfigurations").and_then(|c| c.as_object());
            if let Some(configs) = configs {
                for (key, cfg) in configs {
                    let mut name = String::new();
                    let mut ident = key.clone();
                    if let Some(mode) = cfg.get("mode").and_then(|m| m.as_object()) {
                        if let Some(n) = mode.get("name").and_then(|v| v.as_str()) {
                            name = n.to_string();
                        }
                        ident = mode
                            .get("modeIdentifier")
                            .or_else(|| mode.get("identifier"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(key)
                            .to_string();
                    }
                    if !name.is_empty() {
                        push_mode(&mut out, &mut seen, &ident, &name);
                    }
                }
                if !out.is_empty() {
                    return out.into_iter().take(12).collect();
                }
            }
        }
    }
    out
}

fn list_modes() -> Vec<FocusMode> {
    // Only modes configured on this Mac — never invent Control Center defaults.
    modes_from_db()
}

fn active_from_assertions(modes: &[FocusMode]) -> (String, String) {
    let root = match load_plist_json(&assertions_db_path()) {
        Some(v) => v,
        None => return (String::new(), String::new()),
    };
    let data = root.get("data").and_then(|d| d.as_array());
    let first = data.and_then(|a| a.first()).and_then(|v| v.as_object());
    let records = first
        .and_then(|o| o.get("storeAssertionRecords"))
        .and_then(|r| r.as_array());
    let records = match records {
        Some(r) if !r.is_empty() => r,
        _ => return (String::new(), String::new()),
    };

    let mut best_id = String::new();
    let mut best_ts = -1.0_f64;
    for rec in records {
        let Some(rec) = rec.as_object() else { continue };
        let details = rec.get("assertionDetails").and_then(|d| d.as_object());
        let Some(details) = details else { continue };
        let mid = details
            .get("assertionDetailsModeIdentifier")
            .or_else(|| details.get("modeIdentifier"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if mid.is_empty() {
            continue;
        }
        let ts = rec
            .get("assertionStartDateTimestamp")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if best_id.is_empty() || ts >= best_ts {
            best_id = mid.to_string();
            best_ts = ts;
        }
    }
    if best_id.is_empty() {
        return (String::new(), String::new());
    }
    let found_name = modes
        .iter()
        .find(|m| m.id == best_id)
        .map(|m| m.name.as_str())
        .unwrap_or("");
    let nm = display_name(&best_id, found_name);
    (sanitize(&best_id, ID_MAX), sanitize(&nm, NAME_MAX))
}

pub struct FocusSnapshot {
    pub ok: bool,
    pub active_id: String,
    pub active_name: String,
    pub modes: Vec<FocusMode>,
}

pub fn snapshot() -> FocusSnapshot {
    let modes = list_modes();
    if modes.is_empty() {
        return FocusSnapshot {
            ok: false,
            active_id: String::new(),
            active_name: String::new(),
            modes: vec![],
        };
    }
    let (active_id, mut active_name) = active_from_assertions(&modes);
    if !active_id.is_empty() {
        active_name = display_name(&active_id, &active_name);
    }
    FocusSnapshot {
        ok: true,
        active_id,
        active_name,
        modes: modes.into_iter().take(12).collect(),
    }
}

pub struct FocusCapability {
    last_modes: Vec<FocusMode>,
    last_pushed: Option<String>,
    last_assert_mtime: f64,
    last_modes_mtime: f64,
}

impl FocusCapability {
    pub fn new() -> Self {
        Self {
            last_modes: Vec::new(),
            last_pushed: None,
            last_assert_mtime: 0.0,
            last_modes_mtime: 0.0,
        }
    }

    pub fn last_pushed_line(&self) -> Option<&str> {
        self.last_pushed.as_deref()
    }

    pub fn note_pushed(&mut self, line: &str) {
        self.last_pushed = Some(line.to_string());
        self.last_assert_mtime = file_mtime(&assertions_db_path());
        self.last_modes_mtime = file_mtime(&mode_db_path());
    }

    fn snapshot(&mut self) -> FocusSnapshot {
        let modes = list_modes();
        self.last_modes = modes.clone();
        let snap = snapshot();
        snap
    }

    pub fn build_line(&mut self) -> String {
        let snap = self.snapshot();
        encode_focus_line(snap.ok, &snap.active_id, &snap.active_name, &snap.modes)
    }

    pub fn poll_dirty(&mut self) -> bool {
        let assert_m = file_mtime(&assertions_db_path());
        let modes_m = file_mtime(&mode_db_path());
        if assert_m != self.last_assert_mtime || modes_m != self.last_modes_mtime {
            self.last_assert_mtime = assert_m;
            self.last_modes_mtime = modes_m;
            return true;
        }
        false
    }

    pub fn handle_line(&mut self, text: &str) -> Vec<String> {
        let raw = text.trim();
        if raw.is_empty() {
            return vec![];
        }
        if raw.starts_with("FOCUS?") || raw == "FOCUS\tLIST" {
            return vec![self.build_line()];
        }
        // Focus is observation-only. Never mutate private macOS databases.

        vec![]
    }
}

impl Default for FocusCapability {
    fn default() -> Self {
        Self::new()
    }
}

fn file_mtime(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_fail_matches_python() {
        assert_eq!(encode_focus_line(false, "", "", &[]), "FOCUS\tv1\t0\t\t\t0");
    }

    #[test]
    fn encode_ok_with_modes() {
        let modes = vec![
            FocusMode {
                id: "trabalho".into(),
                name: "Trabalho".into(),
            },
            FocusMode {
                id: "pessoal".into(),
                name: "Pessoal".into(),
            },
        ];
        assert_eq!(
            encode_focus_line(true, "trabalho", "Trabalho", &modes),
            "FOCUS\tv1\t1\ttrabalho\tTrabalho\t2\ttrabalho\tTrabalho\tpessoal\tPessoal"
        );
    }

    #[test]
    fn focus_line_changed_compares_full_wire() {
        let a = "FOCUS\tv1\t1\ttrabalho\tTrabalho\t7\t…";
        let b = "FOCUS\tv1\t1\t\t\t7\t…";
        assert!(focus_line_changed(None, a));
        assert!(!focus_line_changed(Some(a), a));
        assert!(focus_line_changed(Some(b), a));
    }
}

#[cfg(test)]
mod read_only_tests {
    use super::*;
    #[test]
    fn old_mutation_commands_are_ignored() {
        let mut focus = FocusCapability::new();
        assert!(focus.handle_line("FOCUS\tSET\twork").is_empty());
        assert!(focus.handle_line("FOCUS\tOFF").is_empty());
    }
}
