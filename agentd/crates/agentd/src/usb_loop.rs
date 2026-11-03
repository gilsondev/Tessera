//! USB session: handshake, panel commands, capability push heartbeats.

use std::io::Read;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use std::collections::VecDeque;
use std::sync::mpsc::{self, SyncSender};
use tessera_caps_aitokens::{AiTokensCapability, AiTokensFilter};
use tessera_caps_calendar::CalendarCapability;
use tessera_caps_focus::FocusCapability;
use tessera_caps_media::{cover, MediaCapability, MediaInfo, MEDIA_COVER_EDGE};
use tessera_caps_todoist::TodoistCapability;
use tessera_logbuf::SharedLogBuf;
use tessera_protocol::agent_ok_line;
use tessera_usb::{open_tessera_port, write_handshake, write_lines, PortName, UsbError};

use tessera_profile::{read_config, ProfilePaths};

use crate::ipc::UsbLink;
use crate::logutil::{self, describe_usb, widget_of_line};
use crate::mac_events;

/// Firmware marks the Mac offline after ~15s without an `AGENT` line (`agent::kFreshMs`).
const AGENT_HEARTBEAT_INTERVAL_S: u64 = 8;

const ONCE_READ_S: u64 = 2;
const RETRY_S: f64 = 1.5;

type Logs = Option<SharedLogBuf>;

fn load_caps(paths: &ProfilePaths) -> (bool, bool, bool, AiTokensFilter, bool, bool) {
    match read_config(&paths.config_toml()) {
        Ok(c) => (
            c.media,
            c.focus,
            c.aitokens,
            AiTokensFilter {
                cursor: c.aitokens_cursor,
                claude: c.aitokens_claude,
                codex: c.aitokens_codex,
            },
            c.calendar,
            c.todoist,
        ),
        Err(_) => (true, true, true, AiTokensFilter::default(), true, true),
    }
}

fn log_from_panel(logs: &Logs, line: &str) {
    let w = widget_of_line(line);
    let kind = if line.contains('?') { "query" } else { "cmd" };
    // Never echo secrets (Todoist token migration / TOKEN).
    let summary = if line.starts_with("TODOIST_TOKEN\t") || line.starts_with("TOKEN\t") {
        "token (recebido)".to_string()
    } else {
        describe_usb(line)
    };
    logutil::info(logs, format!("usb painel→Mac · {w} · {kind} · {summary}"));
}

fn log_to_panel(logs: &Logs, widget: &str, action: &str, line: &str) {
    logutil::info(
        logs,
        format!(
            "usb Mac→painel · {widget} · {action} · {}",
            describe_usb(line)
        ),
    );
}

pub fn print_candidate_ports() {
    let ports = tessera_usb::list_candidate_ports();
    if ports.is_empty() {
        eprintln!("Tessera: nenhuma porta USB Tessera (usbmodem / Tessera)");
        return;
    }
    eprintln!("Tessera: portas candidatas:");
    for PortName(name) in &ports {
        eprintln!("  {name}");
    }
}

pub fn run_once(path: &str) -> Result<(), UsbError> {
    eprintln!("Tessera: abrindo {path}");
    let mut port = open_tessera_port(path)?;
    write_handshake(&mut port, &agent_ok_line())?;
    eprintln!("Tessera: handshake → {}", agent_ok_line());

    let deadline = Instant::now() + Duration::from_secs(ONCE_READ_S);
    let mut scratch = [0u8; 256];
    while Instant::now() < deadline {
        match port.read(&mut scratch) {
            Ok(0) => continue,
            Ok(n) => {
                let chunk = String::from_utf8_lossy(&scratch[..n]);
                for line in chunk.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        eprintln!("Tessera: ← {line}");
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) => return Err(e.into()),
        }
    }
    eprintln!("Tessera: --once concluído ({ONCE_READ_S}s leitura)");
    Ok(())
}

fn set_usb_link(link: &Option<Arc<RwLock<UsbLink>>>, up: bool) {
    if let Some(usb) = link {
        if let Ok(mut guard) = usb.write() {
            *guard = if up {
                UsbLink::Up
            } else {
                UsbLink::Disconnected
            };
        }
    }
}

/// Every capability owns a worker. Network, Keychain and AppleScript must never
/// delay USB heartbeats or a music change. Bounded queues coalesce repeated queries.
struct Reply {
    lines: Vec<String>,
    track: Option<MediaInfo>,
    cover_key: Option<String>,
}
fn worker<F>(tx: mpsc::Sender<Reply>, mut handle: F) -> SyncSender<String>
where
    F: FnMut(&str) -> Reply + Send + 'static,
{
    let (send, recv) = mpsc::sync_channel::<String>(8);
    std::thread::spawn(move || {
        while let Ok(cmd) = recv.recv() {
            if tx.send(handle(&cmd)).is_err() {
                break;
            }
        }
    });
    send
}
fn lines(lines: Vec<String>) -> Reply {
    Reply {
        lines,
        track: None,
        cover_key: None,
    }
}

pub fn run_session(
    path: &str,
    profile_paths: ProfilePaths,
    usb_link: Option<Arc<RwLock<UsbLink>>>,
    logs: Logs,
) -> Result<(), UsbError> {
    let mut port = open_tessera_port(path)?;
    write_handshake(&mut port, &agent_ok_line())?;
    set_usb_link(&usb_link, true);
    let (reply_tx, reply_rx) = mpsc::channel::<Reply>();
    let mut media = MediaCapability::with_default_watcher();
    let media_tx = worker(reply_tx.clone(), move |cmd| {
        let out = media.handle_line(cmd);
        Reply {
            lines: out,
            track: media.current_info(),
            cover_key: None,
        }
    });
    let mut focus = FocusCapability::new();
    let focus_tx = worker(reply_tx.clone(), move |cmd| lines(focus.handle_line(cmd)));
    let mut ai = AiTokensCapability::with_filter(load_caps(&profile_paths).3);
    let ai_paths = profile_paths.clone();
    let ai_tx = worker(reply_tx.clone(), move |cmd| {
        ai.set_filter(load_caps(&ai_paths).3);
        lines(ai.handle_line(cmd))
    });
    let mut calendar = CalendarCapability::default();
    let calendar_tx = worker(reply_tx.clone(), move |cmd| {
        lines(calendar.handle_line(cmd))
    });
    let mut todoist = TodoistCapability::default();
    let todoist_tx = worker(reply_tx.clone(), move |cmd| lines(todoist.handle_line(cmd)));
    let (cover_tx, cover_rx) = mpsc::sync_channel::<MediaInfo>(1);
    std::thread::spawn(move || {
        let mut cache: VecDeque<(String, Vec<String>)> = VecDeque::new();
        while let Ok(mut info) = cover_rx.recv() {
            while let Ok(newer) = cover_rx.try_recv() {
                info = newer;
            }
            let key = cover::cover_key(&info);
            let cached = cache
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone());
            let chunks = cached.or_else(|| {
                let bytes = cover::fetch_cover_bytes(&info)?;
                let rgb = cover::jpeg_or_png_to_rgb565(&bytes, MEDIA_COVER_EDGE)?;
                Some(cover::encode_cover_chunks(
                    &key,
                    MEDIA_COVER_EDGE,
                    MEDIA_COVER_EDGE,
                    &rgb,
                ))
            });
            if let Some(chunks) = chunks {
                if !cache.iter().any(|(k, _)| *k == key) {
                    cache.push_back((key.clone(), chunks.clone()));
                    if cache.len() > 8 {
                        cache.pop_front();
                    }
                }
                if reply_tx
                    .send(Reply {
                        lines: chunks,
                        track: None,
                        cover_key: Some(key),
                    })
                    .is_err()
                {
                    break;
                }
            }
        }
    });
    let wake_rx = mac_events::start();
    let now = Instant::now();
    let mut deadlines = [now; 5];
    let mut heartbeat = now;
    let mut current_key = String::new();
    let mut sent_cover = String::new();
    let mut cover_retry_at = Instant::now();
    let mut pending_cover: VecDeque<String> = VecDeque::new();
    let mut last_logged = std::collections::HashMap::<String, String>::new();
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    let result = (|| -> Result<(), UsbError> {
        loop {
            let (m, f, a, _, c, t) = load_caps(&profile_paths);
            let enabled = [m, f, a, c, t];
            let workers = [&media_tx, &focus_tx, &ai_tx, &calendar_tx, &todoist_tx];
            let queries = ["MEDIA?", "FOCUS?", "AITOKENS?", "CALENDAR?", "TODOIST?"];
            // Media safety poll is 2s; DNC events request immediately. Others may
            // refresh slowly on their own workers without starving media/USB.
            let periods = [2, 2, 30, 15, 45];
            let (wake_media, wake_focus, wake_theme) = mac_events::drain(&wake_rx);
            if wake_media {
                deadlines[0] = Instant::now();
            }
            if wake_focus {
                deadlines[1] = Instant::now();
            }
            if wake_theme {
                write_lines(
                    &mut port,
                    &[mac_events::theme_line(mac_events::macos_prefers_dark())],
                )?;
            }
            for i in 0..5 {
                if enabled[i] && Instant::now() >= deadlines[i] {
                    let _ = workers[i].try_send(queries[i].into());
                    deadlines[i] = Instant::now() + Duration::from_secs(periods[i]);
                }
            }
            if Instant::now() >= heartbeat {
                write_lines(&mut port, &[agent_ok_line()])?;
                heartbeat = Instant::now() + Duration::from_secs(AGENT_HEARTBEAT_INTERVAL_S);
            }
            match port.read(&mut scratch) {
                Ok(n) => buf.extend_from_slice(&scratch[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e.into()),
            }
            if buf.len() > 64 * 1024 {
                buf.clear();
            }
            while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
                let bytes: Vec<u8> = buf.drain(..=pos).collect();
                let text = String::from_utf8_lossy(&bytes).trim().to_string();
                if text.is_empty() {
                    continue;
                }
                log_from_panel(&logs, &text);
                if text.starts_with("THEME") {
                    write_lines(
                        &mut port,
                        &[mac_events::theme_line(mac_events::macos_prefers_dark())],
                    )?;
                } else if let Some(key) = text.strip_prefix("MEDIA_COVER_OK\t") {
                    if key == current_key && key != sent_cover {
                        sent_cover = key.into();
                        logutil::info(
                            &logs,
                            "usb Mac→painel · media · capa confirmada pelo dispositivo",
                        );
                    }
                } else if text.starts_with("MEDIA_COVER?") {
                    sent_cover.clear();
                    deadlines[0] = Instant::now();
                } else if let Some(i) = ["MEDIA", "FOCUS", "AITOKENS", "CALENDAR", "TODOIST"]
                    .iter()
                    .position(|p| text.starts_with(p))
                {
                    if enabled[i] || text.contains('?') {
                        if workers[i].try_send(text).is_err() {
                            logutil::warn(&logs, "capability busy; query will retry");
                        }
                    }
                }
            }
            while let Ok(reply) = reply_rx.try_recv() {
                if reply.track.is_none()
                    && reply.cover_key.is_none()
                    && reply.lines.iter().any(|l| l.starts_with("MEDIA\tv1\t0"))
                {
                    current_key.clear();
                    sent_cover.clear();
                    pending_cover.clear();
                }
                if let Some(info) = reply.track {
                    let key = cover::cover_key(&info);
                    if key != current_key {
                        current_key = key.clone();
                        sent_cover.clear();
                        pending_cover.clear();
                        cover_retry_at = Instant::now();
                    }
                    if info.state != tessera_caps_media::MediaState::Stopped
                        && key != sent_cover
                        && pending_cover.is_empty()
                        && Instant::now() >= cover_retry_at
                    {
                        if cover_tx.try_send(info).is_ok() {
                            cover_retry_at = Instant::now() + Duration::from_secs(5);
                        }
                    }
                }
                if let Some(key) = reply.cover_key {
                    if key == current_key && key != sent_cover && pending_cover.is_empty() {
                        pending_cover = reply.lines.into();
                    }
                } else if !reply.lines.is_empty() {
                    write_lines(&mut port, &reply.lines)?;
                    for line in &reply.lines {
                        if !line.starts_with("MEDIA_COVER") {
                            let prefix = line.split('\t').next().unwrap_or("");
                            if !matches!(prefix, "MEDIA" | "FOCUS")
                                || last_logged.get(prefix) != Some(line)
                            {
                                log_to_panel(&logs, widget_of_line(line), "push", line);
                                if matches!(prefix, "MEDIA" | "FOCUS") {
                                    last_logged.insert(prefix.into(), line.clone());
                                }
                            }
                        }
                    }
                }
            }
            // Small batches keep commands and metadata responsive during art transfer.
            for _ in 0..4 {
                if let Some(line) = pending_cover.pop_front() {
                    tessera_usb::write_line(&mut port, &line)?;
                    // Confirmation comes from MEDIA_COVER_OK; retry after 5s if absent.
                } else {
                    break;
                }
            }
        }
    })();
    set_usb_link(&usb_link, false);
    result
}

pub fn run_auto(profile_paths: ProfilePaths, usb_link: Option<Arc<RwLock<UsbLink>>>, logs: Logs) {
    logutil::info(&logs, "usb: aguardando painel no cabo…");
    loop {
        let ports = tessera_usb::list_candidate_ports();
        if ports.is_empty() {
            set_usb_link(&usb_link, false);
            std::thread::sleep(Duration::from_secs_f64(RETRY_S));
            continue;
        }
        let PortName(path) = &ports[0];
        match run_session(path, profile_paths.clone(), usb_link.clone(), logs.clone()) {
            Ok(()) => {}
            Err(e) => {
                logutil::warn(&logs, format!("usb: {path} — {e}; reconectando…"));
                set_usb_link(&usb_link, false);
                std::thread::sleep(Duration::from_secs_f64(RETRY_S * 2.0));
            }
        }
    }
}

#[cfg(test)]
mod worker_tests {
    use super::*;
    #[test]
    fn slow_provider_does_not_block_music() {
        let (tx, rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let slow = worker(tx.clone(), move |_| {
            let _ = release_rx.recv();
            lines(vec!["slow".into()])
        });
        let fast = worker(tx, |_| lines(vec!["music".into()]));
        slow.send("refresh".into()).unwrap();
        fast.send("refresh".into()).unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(1)).unwrap().lines,
            vec!["music"]
        );
        release_tx.send(()).unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(1)).unwrap().lines,
            vec!["slow"]
        );
    }
}
