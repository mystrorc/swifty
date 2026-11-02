use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use amane::Service;

static PREV_WORKSPACES: LazyLock<Mutex<HashMap<i64, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static GUARDING: AtomicBool = AtomicBool::new(false);

pub struct MonitorManager;

impl Service for MonitorManager {
    fn new() -> Self {
        // Enforce safety immediately on startup
        sync_and_guard();
        Self
    }

    fn listen() {
        run_monitor_loop();
    }
}

pub fn ipc(arguments: &[String]) -> String {
    match arguments.first().map(|s| s.as_str()) {
        Some("sync" | "apply" | "reload") => {
            sync_and_guard();
            String::from("synced")
        }
        _ => {
            sync_and_guard();
            String::from("ok")
        }
    }
}

pub fn sync_and_guard() {
    if GUARDING.swap(true, Ordering::SeqCst) {
        return;
    }
    struct GuardReset;
    impl Drop for GuardReset {
        fn drop(&mut self) {
            GUARDING.store(false, Ordering::SeqCst);
        }
    }
    let _reset = GuardReset;

    let output = match Command::new("niri").args(["msg", "--json", "outputs"]).output() {
        Ok(out) if out.status.success() => out,
        _ => return,
    };

    let text = match std::str::from_utf8(&output.stdout) {
        Ok(t) => t,
        Err(_) => return,
    };

    let json: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };

    let map = match json.as_object() {
        Some(m) => m,
        None => return,
    };

    let mut internal_name = String::from("eDP-1");
    let mut internal_disabled = false;
    let mut internal_found = false;
    let mut external_outputs: Vec<(String, String, serde_json::Value)> = Vec::new();
    let mut active_outputs: Vec<String> = Vec::new();

    for (name, item) in map {
        let is_internal = name.starts_with("eDP")
            || item.get("make").and_then(|m| m.as_str()).map_or(false, |m| m.contains("Sharp"));

        let is_off = item.get("logical").map_or(true, |l| l.is_null());
        if !is_off {
            active_outputs.push(name.clone());
        }

        if is_internal {
            internal_found = true;
            internal_name = name.clone();
            internal_disabled = is_off;
        } else {
            let make = item.get("make").and_then(|m| m.as_str()).unwrap_or("");
            let model = item.get("model").and_then(|m| m.as_str()).unwrap_or("");
            let serial = item.get("serial").and_then(|s| s.as_str()).unwrap_or("");
            let ident = format!("{make} {model} {serial}").trim().to_string();
            external_outputs.push((name.clone(), ident, item.clone()));
        }
    }

    // Safety Guard 1: Undocked (No external monitors connected)
    if external_outputs.is_empty() {
        if internal_found && internal_disabled {
            eprintln!("amane-shell: Undocked guard: turning ON internal display {internal_name}");
            let _ = Command::new("niri").args(["msg", "output", &internal_name, "on"]).output();
            let _ = Command::new("niri").args(["msg", "output", &internal_name, "scale", "2.0"]).output();
            let _ = Command::new("niri").args(["msg", "output", &internal_name, "mode", "3200x1800@59.982"]).output();
            active_outputs.push(internal_name.clone());
        }
    }

    // Window accessibility guard:
    // Ensure all windows are accessible and visible on active outputs
    if !active_outputs.is_empty() {
        ensure_windows_accessible(&active_outputs);
    }
}

fn ensure_windows_accessible(active_outputs: &[String]) {
    let ws_out = match Command::new("niri").args(["msg", "--json", "workspaces"]).output() {
        Ok(out) if out.status.success() => out,
        _ => return,
    };
    let ws_json: serde_json::Value = match serde_json::from_slice(&ws_out.stdout) {
        Ok(v) => v,
        Err(_) => return,
    };
    let workspaces = match ws_json.as_array() {
        Some(a) => a,
        None => return,
    };

    let win_out = match Command::new("niri").args(["msg", "--json", "windows"]).output() {
        Ok(out) if out.status.success() => out,
        _ => return,
    };
    let win_json: serde_json::Value = match serde_json::from_slice(&win_out.stdout) {
        Ok(v) => v,
        Err(_) => return,
    };
    let windows = match win_json.as_array() {
        Some(a) => a,
        None => return,
    };

    if windows.is_empty() {
        return;
    }

    let mut focused_output = None;
    let mut ws_to_output = HashMap::new();
    let mut ws_is_active = HashMap::new();

    for ws in workspaces {
        if let (Some(id), Some(out)) = (
            ws.get("id").and_then(|i| i.as_i64()),
            ws.get("output").and_then(|o| o.as_str()),
        ) {
            ws_to_output.insert(id, out.to_string());
            let is_act = ws.get("is_active").and_then(|a| a.as_bool()).unwrap_or(false);
            let is_foc = ws.get("is_focused").and_then(|f| f.as_bool()).unwrap_or(false);
            ws_is_active.insert(id, is_act);
            if is_foc && active_outputs.iter().any(|a| a == out) {
                focused_output = Some(out.to_string());
            }
        }
    }

    let primary_target = focused_output
        .as_ref()
        .or_else(|| active_outputs.first())
        .map(|s| s.as_str())
        .unwrap_or("eDP-1");

    let mut prev_map = PREV_WORKSPACES.lock().unwrap();

    for win in windows {
        let win_id = match win.get("id").and_then(|i| i.as_u64()) {
            Some(id) => id,
            None => continue,
        };
        let ws_id = match win.get("workspace_id").and_then(|i| i.as_i64()) {
            Some(id) => id,
            None => continue,
        };

        let current_out = ws_to_output.get(&ws_id).map(|s| s.as_str()).unwrap_or("");
        let is_act = ws_is_active.get(&ws_id).copied().unwrap_or(false);
        let prev_out = prev_map.get(&ws_id).map(|s| s.as_str());

        // Window migration criteria:
        // 1. Current workspace output is not active
        let not_on_active_output = !active_outputs.iter().any(|a| a == current_out);
        // 2. Output where this workspace used to be was turned off, and workspace is inactive
        let prev_output_turned_off = match prev_out {
            Some(po) => !active_outputs.iter().any(|a| a == po) && !is_act,
            None => false,
        };

        if not_on_active_output || prev_output_turned_off {
            let _ = Command::new("niri")
                .args(["msg", "action", "move-window-to-monitor", "--id", &win_id.to_string(), primary_target])
                .output();
        }
    }

    // Keep prev_map updated with current active outputs
    for (id, out) in &ws_to_output {
        if active_outputs.iter().any(|a| a == out) {
            prev_map.insert(*id, out.clone());
        }
    }
}

fn run_monitor_loop() {
    loop {
        let child = Command::new("niri")
            .args(["msg", "--json", "event-stream"])
            .stdout(Stdio::piped())
            .spawn();

        match child {
            Ok(mut process) => {
                if let Some(stdout) = process.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if let Ok(l) = line {
                            if l.contains("WorkspacesChanged") || l.contains("Outputs") || l.contains("output") {
                                sync_and_guard();
                            }
                        } else {
                            break;
                        }
                    }
                }
                let _ = process.kill();
            }
            Err(_) => {
                thread::sleep(Duration::from_secs(5));
                sync_and_guard();
            }
        }

        thread::sleep(Duration::from_secs(2));
    }
}
