use super::elephant;
use super::shortcuts;

use std::cell::RefCell;
use std::env;

use amane::{Apps, Service};

use crate::settings::Settings;

// "> " opens the command palette, "!" lists tmux sessions
const COMMAND_PREFIX: &str = ">";
const TMUX_PREFIX: &str = "!";

#[derive(Clone)]
pub enum Kind {
    // its place in Apps::list()
    App(usize),

    // opens the settings window, on its colors page for the color command
    Settings,
    Colors,

    // opens the wallpaper picker
    Wallpapers,

    // opens the file converter
    Converter,

    // opens the pomodoro timer
    Pomodoro,

    // opens the per-app volume mixer
    Mixer,

    // switches the search to tmux sessions
    TmuxCommand,

    Tmux(String),

    // Elephant provider action
    Elephant {
        provider: String,
        identifier: String,
        action: String,
        value: String,
    },
}

#[derive(Clone)]
pub struct Entry {
    pub name: String,

    // an app's description, shown under its name when turned on
    pub description: Option<String>,

    // shortcut chord from niri config if available
    pub shortcut: Option<String>,

    // a nerd font glyph, for entries that have no app icon
    pub glyph: Option<&'static str>,

    pub kind: Kind,
}

pub fn empty_text(query: &str) -> &'static str {
    if query.starts_with(TMUX_PREFIX) {
        return "No tmux sessions found";
    }

    if query.starts_with(COMMAND_PREFIX) {
        return "No commands found";
    }

    let trimmed = query.trim();
    if query.starts_with("c ") || trimmed.starts_with("clip:") {
        return "No clipboard history found";
    }

    if query.starts_with('?') || query.starts_with("w ") || trimmed.starts_with("web:") {
        return "No web results found";
    }

    "No applications or windows found"
}

thread_local! {
    static CACHED_RESULTS: RefCell<Option<(String, Vec<Entry>)>> = const { RefCell::new(None) };
}

pub fn find(query: &str, sessions: &[String]) -> Vec<Entry> {
    CACHED_RESULTS.with_borrow_mut(|cache| {
        if let Some((cached_query, items)) = cache.as_ref() {
            if cached_query == query {
                return items.clone();
            }
        }

        let items = compute_find(query, sessions);
        *cache = Some((query.to_string(), items.clone()));
        items
    })
}

fn compute_find(query: &str, sessions: &[String]) -> Vec<Entry> {
    if let Some(search) = query.strip_prefix(TMUX_PREFIX) {
        return tmux(&search.trim().to_lowercase(), sessions);
    }

    // with command mode off, ">" is searched for like any other letter
    if let Some(search) = query.strip_prefix(COMMAND_PREFIX) {
        if Settings::read().flag("launcher_commands") {
            return commands(&search.trim().to_lowercase());
        }
    }

    let trimmed = query.trim();

    // Clipboard history: "c " or "clip:"
    if let Some(search) = query.strip_prefix("c ").or_else(|| trimmed.strip_prefix("clip:")) {
        if elephant::is_available() {
            return elephant::query_clipboard(search.trim());
        }
    }

    // Web search: "?" or "w " or "web:"
    if let Some(search) = query
        .strip_prefix('?')
        .or_else(|| query.strip_prefix("w "))
        .or_else(|| trimmed.strip_prefix("web:"))
    {
        if elephant::is_available() {
            return elephant::query_websearch(search.trim());
        }
    }

    let mut found = Vec::new();

    // Calculator promotion (if looks like math)
    if elephant::is_available() && elephant::looks_like_math(trimmed) {
        if let Some(calc_entry) = elephant::query_calc(trimmed) {
            found.push(calc_entry);
        }
    }

    // Standard desktop applications
    found.extend(apps(&trimmed.to_lowercase()));

    // Open windows matching query
    if elephant::is_available() && !trimmed.is_empty() {
        found.extend(elephant::query_windows(trimmed));
    }

    found
}

#[derive(Clone)]
struct CachedApp {
    index: usize,
    name: String,
    name_lower: String,
    exec: String,
    description: Option<String>,
}

thread_local! {
    static CACHED_APPS: RefCell<Option<(usize, Vec<CachedApp>)>> = const { RefCell::new(None) };
}

fn get_cached_apps() -> Vec<CachedApp> {
    let apps = Apps::read();
    let list = apps.list();
    let count = list.len();

    CACHED_APPS.with_borrow_mut(|cache| {
        if let Some((cached_count, items)) = cache.as_ref() {
            if *cached_count == count {
                return items.clone();
            }
        }

        let mut items = Vec::with_capacity(count);
        for (index, app) in list.iter().enumerate() {
            let name = app.name();
            items.push(CachedApp {
                index,
                name: String::from(name),
                name_lower: name.to_lowercase(),
                exec: String::from(app.exec()),
                description: app.description().map(String::from),
            });
        }

        *cache = Some((count, items.clone()));
        items
    })
}

// already sorted by name, so only the filter is left
fn apps(search: &str) -> Vec<Entry> {
    let cached = get_cached_apps();
    let described = Settings::read().flag("launcher_descriptions");

    let mut found = Vec::new();

    for app in cached {
        if !search.is_empty() && !app.name_lower.contains(search) {
            continue;
        }

        let description = if described {
            app.description
        } else {
            None
        };

        let shortcut = shortcuts::get_shortcut(&app.name, &app.exec);

        found.push(Entry {
            name: app.name,
            description,
            shortcut,
            glyph: None,
            kind: Kind::App(app.index),
        });
    }

    let amane_match = search.is_empty()
        || "amane settings".contains(search)
        || "suzuha settings".contains(search)
        || "settings".contains(search)
        || "suzuha".contains(search)
        || "amane".contains(search);

    if amane_match {
        let shortcut = shortcuts::get_shortcut("Amane Settings", "amane settings");
        found.push(Entry {
            name: String::from("Amane Settings"),
            description: if described {
                Some(String::from("Shell settings and preferences"))
            } else {
                None
            },
            shortcut,
            glyph: Some("\u{f0493}"),
            kind: Kind::Settings,
        });
    }

    found
}

// the commands turned on in the settings
fn commands(search: &str) -> Vec<Entry> {
    let all = [
        ("Amane Settings", "\u{f0493}", "command_settings", Kind::Settings),
        ("Color scheme", "\u{f03d8}", "command_colors", Kind::Colors),
        ("Tmux sessions", "\u{f018d}", "command_tmux", Kind::TmuxCommand),
        ("Wallpapers", "\u{f02e9}", "command_wallpapers", Kind::Wallpapers),
        ("Converter", "\u{f04e1}", "command_converter", Kind::Converter),
        ("Pomodoro", "\u{f13ab}", "command_pomodoro", Kind::Pomodoro),
        (
            "Mixer",
            crate::mixer::MIXER_ICON,
            "command_mixer",
            Kind::Mixer,
        ),
    ];

    let settings = Settings::read();

    let mut found = Vec::new();

    for (name, glyph, key, kind) in all {
        if !settings.flag(key) {
            continue;
        }

        if !name.to_lowercase().contains(search) {
            continue;
        }

        let shortcut = shortcuts::get_shortcut(name, name);

        found.push(Entry {
            name: String::from(name),
            description: None,
            shortcut,
            glyph: Some(glyph),
            kind,
        });
    }

    found
}

fn tmux(search: &str, sessions: &[String]) -> Vec<Entry> {
    let mut found = Vec::new();

    for session in sessions {
        if !session.to_lowercase().contains(search) {
            continue;
        }

        found.push(Entry {
            name: session.clone(),
            description: None,
            shortcut: None,
            glyph: None,
            kind: Kind::Tmux(session.clone()),
        });
    }

    found
}

pub fn read_sessions() -> Vec<String> {
    let listed = amane::output("tmux list-sessions -F '#{session_name}'");

    let mut sessions = Vec::new();

    for line in listed.lines() {
        let name = line.trim();

        if !name.is_empty() {
            sessions.push(String::from(name));
        }
    }

    sessions
}

// false when there is no  to attach in
pub fn attach(session: &str) -> bool {
    let Ok(terminal) = env::var("TERMINAL") else {
        return false;
    };

    amane::spawn(&format!("{terminal} -- tmux attach-session -t '{session}'"));

    true
}
