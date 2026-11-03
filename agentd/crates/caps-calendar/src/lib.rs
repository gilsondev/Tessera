//! CALENDAR capability — native macOS calendars; only selected snapshots
//! and calendar catalog metadata are sent to the panel over USB.

pub mod native;

use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) const EVENT_LIMIT: usize = 12;
const TITLE_LIMIT: usize = 47;
const LOCATION_LIMIT: usize = 31;
/// Refresh native calendar snapshots without blocking the USB transport.
pub const CALENDAR_REFRESH_INTERVAL_S: u64 = 120;
/// USB heartbeat for calendar snapshot (panel drops stale views).
pub const CALENDAR_PUSH_INTERVAL_S: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct CalendarEvent {
    pub start_hhmm: String,
    pub end_hhmm: String,
    pub all_day: bool,
    pub title: String,
    pub location: String,
    pub duration_min: u32,
    pub color: String,
    pub day_offset: u8,
}

fn wire_field(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .map(|c| match c {
            '\t' | '\r' | '\n' => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn hhmm(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() == 4 && bytes.iter().all(u8::is_ascii_digit) {
        value.to_string()
    } else {
        "0000".to_string()
    }
}

fn color(value: &str) -> String {
    let value = value.trim().trim_start_matches('#');
    if value.len() == 6 && value.bytes().all(|c| c.is_ascii_hexdigit()) {
        value.to_ascii_uppercase()
    } else {
        "1A73E8".to_string()
    }
}

/// Encodes the legacy `CALENDAR\tv1` snapshot. Events are capped to the
/// device's fixed 12-event cache and invalid offsets are dropped.
pub fn encode_calendar_line(linked: bool, events: &[CalendarEvent]) -> String {
    let mut fields = vec![
        "CALENDAR".to_string(),
        "v1".to_string(),
        "1".to_string(),
        if linked { "1" } else { "0" }.to_string(),
    ];
    if !linked {
        return fields.join("\t");
    }
    for event in events
        .iter()
        .filter(|e| e.day_offset <= 14)
        .take(EVENT_LIMIT)
    {
        let (start, end, all_day) = if event.all_day {
            ("0000".to_string(), "0000".to_string(), "1")
        } else {
            (hhmm(&event.start_hhmm), hhmm(&event.end_hhmm), "0")
        };
        fields.extend([
            start,
            end,
            all_day.to_string(),
            wire_field(&event.title, TITLE_LIMIT),
            wire_field(&event.location, LOCATION_LIMIT),
            event.duration_min.to_string(),
            color(&event.color),
            event.day_offset.to_string(),
        ]);
    }
    fields.join("\t")
}

pub fn calendar_line_changed(last_pushed: Option<&str>, encoded: &str) -> bool {
    last_pushed != Some(encoded)
}

/// Calendar days between the cache timestamp and today (0 when written today).
pub fn cache_age_in_days(fetched_at: &str) -> Option<i64> {
    let written = chrono::DateTime::parse_from_rfc3339(fetched_at).ok()?;
    let days = (chrono::Local::now().date_naive() - written.date_naive()).num_days();
    Some(days.max(0))
}

#[derive(Debug, Clone)]
pub struct CalendarPaths {
    pub root: PathBuf,
}
impl CalendarPaths {
    pub fn default_home() -> Self {
        Self {
            root: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join(".tessera/calendar"),
        }
    }
    pub fn selection_json(&self) -> PathBuf {
        self.root.join("selection.json")
    }
    pub fn events_cache_json(&self) -> PathBuf {
        self.root.join("events_cache.json")
    }
}
pub struct CalendarCapability {
    paths: CalendarPaths,
    last_pushed: Option<String>,
    last_refresh: Instant,
    cached: Vec<CalendarEvent>,
    linked: bool,
    catalog: serde_json::Value,
    failed: bool,
}
impl CalendarCapability {
    pub fn new(paths: CalendarPaths) -> Self {
        Self {
            paths,
            last_pushed: None,
            last_refresh: Instant::now()
                .checked_sub(Duration::from_secs(120))
                .unwrap(),
            cached: vec![],
            linked: false,
            catalog: serde_json::Value::Null,
            failed: false,
        }
    }
    pub fn default_home() -> Self {
        Self::new(CalendarPaths::default_home())
    }
    pub fn last_pushed_line(&self) -> Option<&str> {
        self.last_pushed.as_deref()
    }
    pub fn note_pushed(&mut self, line: &str) {
        self.last_pushed = Some(line.into());
    }
    pub fn refresh_cache_if_due(&mut self) {
        if self.last_refresh.elapsed() < Duration::from_secs(15) {
            return;
        }
        self.last_refresh = Instant::now();
        match native::run(&self.paths, "snapshot") {
            Ok(value) => self.accept_snapshot(value),
            Err(_) => self.failed = true,
        }
    }
    fn accept_snapshot(&mut self, value: serde_json::Value) {
        self.failed = false;
        self.last_refresh = Instant::now();
        self.linked = value["permission"] == "authorized";
        self.cached = if self.linked {
            serde_json::from_value(value["events"].clone()).unwrap_or_default()
        } else {
            vec![]
        };
        self.catalog = value;
    }

    pub fn build_events_line(&mut self) -> String {
        self.refresh_cache_if_due();
        if self.failed {
            return String::new();
        }
        encode_calendar_line(self.linked, &self.cached)
    }
    pub fn build_catalog_lines(&self) -> Vec<String> {
        if self.failed {
            return vec!["CALENDAR_CATALOG\tv1\t0\tCalendários indisponíveis".into()];
        }
        let selected = native::selection(&self.paths);
        let mut lines = vec![format!(
            "CALENDAR_CATALOG\tv1\t{}\t{}",
            if self.linked { "1" } else { "0" },
            if self.linked {
                "Calendários do Mac"
            } else {
                "Permita Calendários no Agente"
            }
        )];
        if let Some(rows) = self.catalog["calendars"].as_array() {
            for row in rows.iter().take(24) {
                let id = row["id"].as_str().unwrap_or("");
                // Account is the local source name, not necessarily an email address.
                lines.push(format!(
                    "CALENDAR_CATALOG\tv1\t1\t+\t{}\t{}\t{}\t{}\t{}",
                    wire_field(row["account"].as_str().unwrap_or("No Mac"), 63),
                    wire_field(id, 127),
                    wire_field(row["name"].as_str().unwrap_or(""), 63),
                    row["color"].as_str().unwrap_or("1A73E8"),
                    if selected.iter().any(|s| s == id) {
                        "1"
                    } else {
                        "0"
                    }
                ));
            }
        }
        lines
    }
    pub fn handle_line(&mut self, text: &str) -> Vec<String> {
        if text.starts_with("CALENDAR_AUTH?") {
            let _ = native::start(self.paths.clone());
            return vec!["CALENDAR_AUTH\tv1\t1\tPermita Calendários no Mac".into()];
        }
        if let Some(ids) = text.strip_prefix("CALENDAR_SEL\t") {
            let ids: Vec<_> = ids.split('\t').filter(|s| !s.is_empty()).collect();
            let _ = native::private_json(&self.paths.selection_json(), &serde_json::json!(ids));
            self.last_refresh = Instant::now()
                .checked_sub(Duration::from_secs(120))
                .unwrap();
        }
        if text.starts_with("CALENDAR") {
            let snapshot = self.build_events_line();
            let mut lines = if snapshot.is_empty() {
                vec![]
            } else {
                vec![snapshot]
            };
            lines.extend(self.build_catalog_lines());
            lines.push(format!(
                "CALENDAR_AUTH\tv1\t0\t{}",
                if self.linked {
                    "Calendários do Mac"
                } else {
                    "Permita Calendários no Agente"
                }
            ));
            return lines;
        }
        vec![]
    }
}
impl Default for CalendarCapability {
    fn default() -> Self {
        Self::default_home()
    }
}

#[cfg(test)]
mod native_snapshot_tests {
    use super::*;
    #[test]
    fn permission_revocation_clears_events() {
        let temp = tempfile::tempdir().unwrap();
        let mut cal = CalendarCapability::new(CalendarPaths {
            root: temp.path().into(),
        });
        cal.accept_snapshot(serde_json::json!({"permission":"authorized","calendars":[{"id":"local","name":"Pessoal","account":"iCloud","color":"AABBCC"}],"events":[{"start_hhmm":"0900","end_hhmm":"1000","all_day":false,"title":"Synthetic event","location":"","duration_min":60,"color":"AABBCC","day_offset":1}]}));
        assert!(cal.build_events_line().contains("Synthetic event"));
        assert!(cal
            .build_catalog_lines()
            .iter()
            .any(|l| l.contains("iCloud")));
        cal.accept_snapshot(serde_json::json!({"permission":"denied","calendars":[],"events":[]}));
        assert_eq!(cal.build_events_line(), "CALENDAR\tv1\t1\t0");
    }
    #[test]
    fn failed_collection_does_not_refresh_old_events_as_new() {
        let temp = tempfile::tempdir().unwrap();
        let mut cal = CalendarCapability::new(CalendarPaths {
            root: temp.path().into(),
        });
        cal.last_refresh = Instant::now();
        cal.failed = true;
        assert!(cal.build_events_line().is_empty());
        assert!(cal.build_catalog_lines()[0].contains("indisponíveis"));
    }
}
