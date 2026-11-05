use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{LazyLock, Mutex};

use amane::Service;

static CACHED_WIDTHS: LazyLock<Mutex<HashMap<String, f32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HalfScreen {
    Left,
    Right,
}

#[derive(Default)]
pub struct DesktopWindows {
    // Monitor name -> occupied half screen if any
    occupied: HashMap<String, Option<HalfScreen>>,
}

impl Service for DesktopWindows {
    fn new() -> Self {
        let mut initial = Self::default();
        initial.occupied = detect_occupied_windows();
        initial
    }

    fn listen() {
        loop {
            // Initial sync
            let new_map = detect_occupied_windows();
            if Self::read().occupied != new_map {
                let mut state = Self::write();
                state.occupied = new_map;
            }

            // Spawn continuous event-stream listener
            let child = Command::new("niri")
                .args(["msg", "--json", "event-stream"])
                .stdout(Stdio::piped())
                .spawn();

            match child {
                Ok(mut process) => {
                    if let Some(stdout) = process.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let mut last_check = Instant::now() - Duration::from_secs(1);
                        for line in reader.lines() {
                            if let Ok(l) = line {
                                if l.contains("OutputsChanged") {
                                    if let Ok(mut cw) = CACHED_WIDTHS.lock() {
                                        cw.clear();
                                    }
                                }
                                if l.contains("Window") || l.contains("Workspace") {
                                    let now = Instant::now();
                                    if now.duration_since(last_check) < Duration::from_millis(200) {
                                        continue;
                                    }
                                    last_check = now;
                                    let new_map = detect_occupied_windows();
                                    if Self::read().occupied != new_map {
                                        let mut state = Self::write();
                                        state.occupied = new_map;
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    let _ = process.kill();
                }
                Err(_) => {
                    thread::sleep(Duration::from_secs(3));
                }
            }

            thread::sleep(Duration::from_secs(2));
        }
    }
}

impl DesktopWindows {
    pub fn occupied_half(&self, monitor: &str) -> Option<HalfScreen> {
        self.occupied.get(monitor).copied().flatten()
    }
}

fn detect_occupied_windows() -> HashMap<String, Option<HalfScreen>> {
    let mut results = HashMap::new();

    let ws_out = match Command::new("niri").args(["msg", "--json", "workspaces"]).output() {
        Ok(out) if out.status.success() => out,
        _ => return results,
    };
    let ws_json: serde_json::Value = match serde_json::from_slice(&ws_out.stdout) {
        Ok(v) => v,
        Err(_) => return results,
    };

    let win_out = match Command::new("niri").args(["msg", "--json", "windows"]).output() {
        Ok(out) if out.status.success() => out,
        _ => return results,
    };
    let win_json: serde_json::Value = match serde_json::from_slice(&win_out.stdout) {
        Ok(v) => v,
        Err(_) => return results,
    };

    let Some(workspaces) = ws_json.as_array() else { return results; };
    let Some(windows) = win_json.as_array() else { return results; };

    // Map monitor_name -> active_workspace_id
    let mut mon_active_ws = HashMap::new();
    for ws in workspaces {
        if ws.get("is_active").and_then(|a| a.as_bool()).unwrap_or(false) {
            if let (Some(out), Some(id)) = (
                ws.get("output").and_then(|o| o.as_str()),
                ws.get("id").and_then(|i| i.as_i64()),
            ) {
                mon_active_ws.insert(out.to_string(), id);
            }
        }
    }

    // Map monitor_name -> logical_width (cached to avoid forking on every check)
    let mon_width = {
        let mut cached = CACHED_WIDTHS.lock().unwrap();
        if cached.is_empty() {
            if let Ok(out_out) = Command::new("niri").args(["msg", "--json", "outputs"]).output() {
                if let Ok(out_json) = serde_json::from_slice::<serde_json::Value>(&out_out.stdout) {
                    if let Some(outputs) = out_json.as_object() {
                        for (name, out) in outputs {
                            if let Some(w) = out.get("logical").and_then(|l| l.get("width")).and_then(|w| w.as_f64()) {
                                cached.insert(name.clone(), w as f32);
                            }
                        }
                    }
                }
            }
        }
        cached.clone()
    };

    for (mon, ws_id) in mon_active_ws {
        let width = mon_width.get(&mon).copied().unwrap_or(1920.0);

        // Windows on this workspace
        let ws_wins: Vec<&serde_json::Value> = windows
            .iter()
            .filter(|w| w.get("workspace_id").and_then(|i| i.as_i64()) == Some(ws_id))
            .collect();

        if ws_wins.is_empty() {
            results.insert(mon, None);
            continue;
        }

        // Check non-floating windows
        let tiled_wins: Vec<&serde_json::Value> = ws_wins
            .iter()
            .copied()
            .filter(|w| !w.get("is_floating").and_then(|f| f.as_bool()).unwrap_or(false))
            .collect();

        if !tiled_wins.is_empty() {
            let mut cols = HashMap::new();
            for w in tiled_wins {
                if let Some(layout) = w.get("layout") {
                    let col_idx = layout
                        .get("pos_in_scrolling_layout")
                        .and_then(|p| p.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|c| c.as_i64())
                        .unwrap_or(1);

                    let tile_w = layout
                        .get("tile_size")
                        .and_then(|s| s.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|w| w.as_f64())
                        .unwrap_or(width as f64) as f32;

                    cols.insert(col_idx, tile_w);
                }
            }

            if cols.len() == 1 {
                let (_, tile_w) = cols.into_iter().next().unwrap();
                if tile_w < width * 0.70 {
                    // Column 1 in Niri starts at left edge
                    results.insert(mon, Some(HalfScreen::Left));
                } else {
                    results.insert(mon, None);
                }
            } else {
                results.insert(mon, None);
            }
        } else {
            // Only floating windows
            let mut xs = Vec::new();
            for w in &ws_wins {
                if let Some(pos) = w.get("layout")
                    .and_then(|l| l.get("tile_pos_in_workspace_view"))
                    .and_then(|p| p.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|x| x.as_f64())
                {
                    xs.push(pos as f32);
                }
            }

            if !xs.is_empty() && xs.iter().all(|&x| x < width * 0.45) {
                results.insert(mon, Some(HalfScreen::Left));
            } else if !xs.is_empty() && xs.iter().all(|&x| x >= width * 0.45) {
                results.insert(mon, Some(HalfScreen::Right));
            } else {
                results.insert(mon, None);
            }
        }
    }

    results
}
