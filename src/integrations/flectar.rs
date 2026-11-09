use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use super::{hex, home};
use crate::theme::Theme;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalendarEvent {
    pub id: i64,
    pub summary: String,
    pub location: Option<String>,
    pub starts_at: i64, // Unix epoch seconds
    pub ends_at: Option<i64>,
    pub all_day: bool,
    pub join_url: Option<String>,
}

static CACHED_EVENTS: LazyLock<Mutex<(Option<SystemTime>, Vec<CalendarEvent>)>> =
    LazyLock::new(|| Mutex::new((None, Vec::new())));

fn calendar_db_path() -> PathBuf {
    PathBuf::from(format!("{}/.local/share/flectar-mail/flectar-calendar.db", home()))
}

/*
 * Reads all active calendar events directly from Flectar Mail's SQLite database.
 * Uses mtime caching to avoid re-querying when the database has not changed.
 */
pub fn get_events() -> Vec<CalendarEvent> {
    let path = calendar_db_path();
    if !path.exists() {
        return Vec::new();
    }

    let mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
    {
        if let Ok(cache) = CACHED_EVENTS.lock() {
            if cache.0.is_some() && cache.0 == mtime {
                return cache.1.clone();
            }
        }
    }

    let py_code = r#"
import sqlite3, os, json, sys

db_path = sys.argv[1]
try:
    con = sqlite3.connect(db_path, timeout=1.0)
    cur = con.cursor()
    tbl = cur.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='calendar_events';").fetchone()
    if not tbl:
        print("[]")
        sys.exit(0)

    rows = cur.execute("""
        SELECT id, summary, location, starts_at, ends_at, all_day, join_url
        FROM calendar_events
        WHERE deleted = 0
        ORDER BY starts_at ASC;
    """).fetchall()

    events = []
    for r in rows:
        starts = r[3]
        if starts > 100_000_000_000:
            starts = starts // 1000
        ends = r[4]
        if ends is not None and ends > 100_000_000_000:
            ends = ends // 1000

        events.append({
            "id": r[0],
            "summary": r[1] or "Untitled Event",
            "location": r[2],
            "starts_at": starts,
            "ends_at": ends,
            "all_day": bool(r[5]),
            "join_url": r[6],
        })
    print(json.dumps(events))
    con.close()
except Exception:
    print("[]")
"#;

    let output = Command::new("python3")
        .args(["-c", py_code, path.to_str().unwrap_or_default()])
        .output();

    let events: Vec<CalendarEvent> = match output {
        Ok(out) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout);
            serde_json::from_str(s.trim()).unwrap_or_default()
        }
        _ => Vec::new(),
    };

    if let Ok(mut cache) = CACHED_EVENTS.lock() {
        *cache = (mtime, events.clone());
    }

    events
}

pub fn events_for_day(offset_secs: i64, year: i64, month: i64, day: i64) -> Vec<CalendarEvent> {
    let target_days = crate::clock::days_since_1970(year, month, day);
    let all = get_events();

    all.into_iter()
        .filter(|event| {
            let start_local = event.starts_at + offset_secs;
            let start_day = start_local.div_euclid(86400);

            let end_day = if let Some(ends) = event.ends_at {
                let end_local = ends + offset_secs;
                end_local.div_euclid(86400)
            } else {
                start_day
            };

            target_days >= start_day && target_days <= end_day
        })
        .collect()
}

pub fn has_event_on(offset_secs: i64, year: i64, month: i64, day: i64) -> bool {
    !events_for_day(offset_secs, year, month, day).is_empty()
}

/*
 * Exports the dynamic shell theme directly into Flectar Mail's SQLite database
 * (~/.local/share/flectar-mail/flectar-mail.db) so Flectar Mail's colors match the desktop wallpaper.
 */
pub fn export(theme: &Theme) {
    let db_path = format!("{}/.local/share/flectar-mail/flectar-mail.db", home());
    if !Path::new(&db_path).exists() {
        return;
    }

    let dark_primary = hex(theme.accent);
    let dark_bg = hex(theme.background);
    let dark_surface = hex(theme.surface);
    let dark_text = hex(theme.text);
    let dark_border = hex(theme.border);

    let light_primary = hex(theme.accent);
    let light_bg = "#f2f2f0";
    let light_surface = "#ffffff";
    let light_text = "#202120";
    let light_border = "#d9d9d6";

    let py_code = r#"
import sqlite3, json, sys

db_path, dp, dbg, ds, dt, db, lp, lbg, ls, lt, lb = sys.argv[1:12]

try:
    con = sqlite3.connect(db_path)
    cur = con.cursor()
    row = cur.execute("SELECT value FROM app_settings WHERE key='settings';").fetchone()
    if not row:
        con.close()
        sys.exit(0)

    s = json.loads(row[0])
    s['themePreset'] = 'custom'
    s['theme'] = 'system'
    ct = s.get('customTheme', {})
    ct['darkPrimary'] = dp
    ct['darkPageBackground'] = dbg
    ct['darkSurface'] = ds
    ct['darkText'] = dt
    ct['darkBorder'] = db
    ct['lightPrimary'] = lp
    ct['lightPageBackground'] = lbg
    ct['lightSurface'] = ls
    ct['lightText'] = lt
    ct['lightBorder'] = lb
    s['customTheme'] = ct

    cur.execute("UPDATE app_settings SET value=? WHERE key='settings';", (json.dumps(s),))
    con.commit()
    con.close()
except Exception:
    pass
"#;

    let _ = Command::new("python3")
        .args([
            "-c",
            py_code,
            &db_path,
            &dark_primary,
            &dark_bg,
            &dark_surface,
            &dark_text,
            &dark_border,
            &light_primary,
            light_bg,
            light_surface,
            light_text,
            light_border,
        ])
        .status();
}
