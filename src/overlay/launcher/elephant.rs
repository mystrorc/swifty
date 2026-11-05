use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use amane::Service;
use serde::Deserialize;

use crate::settings::Settings;
use super::results::{Entry, Kind};

pub fn socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("elephant/elephant.sock")
    } else {
        PathBuf::from("/run/user/1000/elephant/elephant.sock")
    }
}

pub fn is_available() -> bool {
    if !Settings::read().flag("launcher_elephant") {
        return false;
    }
    socket_path().exists()
}

pub fn looks_like_math(query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return false;
    }
    let has_math_op = q.chars().any(|c| matches!(c, '+' | '*' | '/' | '%' | '^' | '(' | ')'));
    let has_subtraction = q.contains('-') && q.chars().filter(|c| c.is_ascii_digit()).count() >= 2;
    let has_func = q.starts_with("sqrt")
        || q.starts_with("sin")
        || q.starts_with("cos")
        || q.starts_with("tan")
        || q.starts_with("log");

    has_math_op || has_subtraction || has_func
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ElephantResponse {
    pub query: Option<String>,
    pub item: Option<ElephantItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ElephantItem {
    #[serde(default)]
    pub identifier: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub subtext: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub score: i64,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub state: Vec<String>,
    #[serde(default)]
    pub preview: Option<String>,
}

pub fn query_raw(provider_query: &str) -> Vec<ElephantItem> {
    let Ok(output) = Command::new("elephant")
        .arg("query")
        .arg("--json")
        .arg(provider_query)
        .output()
    else {
        return Vec::new();
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut items = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(resp) = serde_json::from_str::<ElephantResponse>(line) {
            if let Some(item) = resp.item {
                items.push(item);
            }
        }
    }

    items
}

pub fn query_calc(query: &str) -> Option<Entry> {
    if !Settings::read().flag("launcher_elephant_calc") {
        return None;
    }

    let query_str = format!("calc;{};1", query.trim());
    let items = query_raw(&query_str);

    for item in items {
        let is_current = item.state.iter().any(|s| s == "current");
        if is_current && !item.text.is_empty() {
            let formula = item.subtext.unwrap_or_else(|| query.to_string());
            return Some(Entry {
                name: item.text.clone(),
                description: Some(format!("= {} • Enter to copy", formula)),
                shortcut: None,
                glyph: Some("\u{f0aa6}"), // 󰪚
                kind: Kind::Elephant {
                    provider: "calc".to_string(),
                    identifier: item.identifier,
                    action: "copy".to_string(),
                    value: item.text,
                },
            });
        }
    }

    None
}

thread_local! {
    static CACHED_WINDOWS: RefCell<Option<(Instant, Vec<ElephantItem>)>> = const { RefCell::new(None) };
    static CACHED_CLIPBOARD: RefCell<Option<(Instant, Vec<ElephantItem>)>> = const { RefCell::new(None) };
}

pub fn get_open_windows() -> Vec<ElephantItem> {
    CACHED_WINDOWS.with_borrow_mut(|cache| {
        let now = Instant::now();
        if let Some((fetched_at, items)) = cache.as_ref() {
            if now.duration_since(*fetched_at) < Duration::from_millis(500) {
                return items.clone();
            }
        }
        let items = query_raw("windows;;25");
        *cache = Some((now, items.clone()));
        items
    })
}

pub fn query_windows(filter: &str) -> Vec<Entry> {
    if !Settings::read().flag("launcher_elephant_windows") {
        return Vec::new();
    }

    let filter_lower = filter.trim().to_lowercase();
    let windows = get_open_windows();
    let mut results = Vec::new();

    for win in windows {
        let title_match = win.text.to_lowercase().contains(&filter_lower);
        let app_match = win
            .subtext
            .as_ref()
            .map_or(false, |s| s.to_lowercase().contains(&filter_lower));

        if filter_lower.is_empty() || title_match || app_match {
            let desc = match &win.subtext {
                Some(app) => format!("{} • Switch window", app),
                None => "Switch window".to_string(),
            };

            results.push(Entry {
                name: win.text,
                description: Some(desc),
                shortcut: None,
                glyph: Some("\u{f05af}"), // 󰖯
                kind: Kind::Elephant {
                    provider: "windows".to_string(),
                    identifier: win.identifier,
                    action: "focus".to_string(),
                    value: String::new(),
                },
            });
        }
    }

    results
}

pub fn get_clipboard_history() -> Vec<ElephantItem> {
    CACHED_CLIPBOARD.with_borrow_mut(|cache| {
        let now = Instant::now();
        if let Some((fetched_at, items)) = cache.as_ref() {
            if now.duration_since(*fetched_at) < Duration::from_millis(500) {
                return items.clone();
            }
        }
        let items = query_raw("clipboard;;30");
        *cache = Some((now, items.clone()));
        items
    })
}

pub fn query_clipboard(filter: &str) -> Vec<Entry> {
    if !Settings::read().flag("launcher_elephant_clipboard") {
        return Vec::new();
    }

    let filter_lower = filter.trim().to_lowercase();
    let items = get_clipboard_history();
    let mut results = Vec::new();

    for item in items {
        let text_clean = item.text.replace('\n', " ").trim().to_string();
        if !filter_lower.is_empty() && !text_clean.to_lowercase().contains(&filter_lower) {
            continue;
        }

        let desc = match &item.subtext {
            Some(ts) => format!("{} • Enter to copy", ts),
            None => "Clipboard history • Enter to copy".to_string(),
        };

        let display_name = if text_clean.chars().count() > 80 {
            let s: String = text_clean.chars().take(77).collect();
            format!("{}...", s)
        } else {
            text_clean
        };

        results.push(Entry {
            name: display_name,
            description: Some(desc),
            shortcut: None,
            glyph: Some("\u{f014d}"), // 󰅍
            kind: Kind::Elephant {
                provider: "clipboard".to_string(),
                identifier: item.identifier,
                action: "copy".to_string(),
                value: item.text,
            },
        });
    }

    results
}

pub fn query_websearch(query: &str) -> Vec<Entry> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }

    let items = query_raw(&format!("websearch;{};5", query));
    let mut results = Vec::new();

    if items.is_empty() {
        results.push(Entry {
            name: format!("Search web for \"{}\"", query),
            description: Some("Google Search • Enter to search".to_string()),
            shortcut: None,
            glyph: Some("\u{f059f}"), // 󰖟
            kind: Kind::Elephant {
                provider: "websearch".to_string(),
                identifier: "0".to_string(),
                action: "search".to_string(),
                value: query.to_string(),
            },
        });
    } else {
        for item in items {
            let engine = if item.text.is_empty() {
                "Web Search"
            } else {
                &item.text
            };
            results.push(Entry {
                name: format!("{} for \"{}\"", engine, query),
                description: Some(format!("{} • Enter to search", engine)),
                shortcut: None,
                glyph: Some("\u{f059f}"), // 󰖟
                kind: Kind::Elephant {
                    provider: "websearch".to_string(),
                    identifier: item.identifier,
                    action: "search".to_string(),
                    value: query.to_string(),
                },
            });
        }
    }

    results
}

pub fn activate(provider: &str, identifier: &str, action: &str, value: &str) {
    let activate_arg = format!("{};{};{};{};0", provider, identifier, action, value);

    let _ = Command::new("elephant")
        .arg("activate")
        .arg(&activate_arg)
        .spawn();

    match provider {
        "calc" | "clipboard" => {
            if !value.is_empty() {
                if let Ok(mut child) = Command::new("wl-copy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                {
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        let _ = stdin.write_all(value.as_bytes());
                    }
                    let _ = child.wait();
                }
            }
        }
        _ => {}
    }
}
