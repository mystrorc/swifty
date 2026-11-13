use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use serde::{Deserialize, Serialize};

use amane::{
    Button, Center, Column, Padding, Point, Pointer, Rectangle, Row, Scroll,
    Stack, Start, Text, Weight, Widget, children,
};

use super::super::button::{self, Style};
use super::super::page::Page;
use super::super::{high_surface, row};
use crate::fonts;
use crate::theme::{self, Theme};

pub static ISO_10_FREQUENCIES: [f32; 10] = [
    31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub static ISO_10_LABELS: [&str; 10] = [
    "31.5", "63", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
];

pub static ISO_20_FREQUENCIES: [f32; 20] = [
    31.5, 45.0, 63.0, 90.0, 125.0, 180.0, 250.0, 355.0, 500.0, 710.0,
    1000.0, 1400.0, 2000.0, 2800.0, 4000.0, 5600.0, 8000.0, 11200.0, 16000.0, 22400.0,
];

pub static ISO_20_LABELS: [&str; 20] = [
    "31.5", "45", "63", "90", "125", "180", "250", "355", "500", "710",
    "1k", "1.4k", "2k", "2.8k", "4k", "5.6k", "8k", "11.2k", "16k", "22.4k",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub description: String,
    pub preamp_db: f32,
    pub is_20_band: bool,
    pub gains_10: [f32; 10],
    pub gains_20: [f32; 20],
}

pub fn builtin_presets() -> Vec<Preset> {
    vec![
        Preset {
            name: "Flat".to_string(),
            description: "Zero gain across all frequency bands".to_string(),
            preamp_db: 0.0,
            is_20_band: false,
            gains_10: [0.0; 10],
            gains_20: [0.0; 20],
        },
        Preset {
            name: "Bass Boost".to_string(),
            description: "Elevated low end for extra punch and warmth".to_string(),
            preamp_db: -4.0,
            is_20_band: false,
            gains_10: [7.0, 6.0, 5.0, 3.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            gains_20: [7.0, 6.5, 6.0, 5.5, 5.0, 4.0, 3.0, 2.0, 1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        Preset {
            name: "Vocal Clarity".to_string(),
            description: "Enhanced mid-range frequencies for speech and vocals".to_string(),
            preamp_db: -2.0,
            is_20_band: false,
            gains_10: [-2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0, -1.0],
            gains_20: [-2.0, -1.5, -1.0, -0.5, 0.0, 1.0, 2.0, 3.0, 4.0, 4.0, 4.0, 3.5, 3.0, 2.0, 1.0, 0.5, 0.0, -0.5, -1.0, -1.0],
        },
        Preset {
            name: "Treble Boost".to_string(),
            description: "Brightened highs for enhanced detail and sparkle".to_string(),
            preamp_db: -4.0,
            is_20_band: false,
            gains_10: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 3.0, 5.0, 6.0, 7.0],
            gains_20: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 5.5, 6.0, 6.5, 7.0, 7.0],
        },
        Preset {
            name: "Rock".to_string(),
            description: "Classic V-curve with punchy lows and crisp highs".to_string(),
            preamp_db: -3.0,
            is_20_band: false,
            gains_10: [5.0, 4.0, 2.0, -1.0, -2.0, -1.0, 2.0, 4.0, 5.0, 6.0],
            gains_20: [5.0, 4.5, 4.0, 3.0, 2.0, 0.5, -1.0, -1.5, -2.0, -1.5, -1.0, 0.5, 2.0, 3.0, 4.0, 4.5, 5.0, 5.5, 6.0, 6.0],
        },
        Preset {
            name: "Electronic".to_string(),
            description: "Deep sub-bass boost with sculpted highs for EDM".to_string(),
            preamp_db: -4.0,
            is_20_band: false,
            gains_10: [6.0, 5.0, 3.0, 0.0, -2.0, 2.0, 1.0, 3.0, 5.0, 6.0],
            gains_20: [6.0, 5.5, 5.0, 4.0, 3.0, 1.5, 0.0, -1.0, -2.0, 0.0, 2.0, 1.5, 1.0, 2.0, 3.0, 4.0, 5.0, 5.5, 6.0, 6.0],
        },
        Preset {
            name: "Classical".to_string(),
            description: "Subtle organic enhancement for orchestral balance".to_string(),
            preamp_db: -2.0,
            is_20_band: false,
            gains_10: [4.0, 3.0, 2.0, 1.0, -1.0, -1.0, 0.0, 2.0, 3.0, 4.0],
            gains_20: [4.0, 3.5, 3.0, 2.5, 2.0, 1.5, 1.0, 0.0, -1.0, -1.0, -1.0, -0.5, 0.0, 1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 4.0],
        },
        Preset {
            name: "Acoustic".to_string(),
            description: "Warm lows and transparent air for unplugged instruments".to_string(),
            preamp_db: -2.0,
            is_20_band: false,
            gains_10: [3.0, 2.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 2.0],
            gains_20: [3.0, 2.5, 2.0, 1.5, 1.0, 1.0, 1.0, 1.5, 2.0, 2.0, 2.0, 2.5, 3.0, 3.0, 3.0, 3.0, 3.0, 2.5, 2.0, 2.0],
        },
        // AutoEQ profiles
        Preset {
            name: "Sony WH-1000XM3".to_string(),
            description: "AutoEq Harman Target (oratory1990) 20-band calibrated profile".to_string(),
            preamp_db: -5.3,
            is_20_band: true,
            gains_10: [-10.6, -10.8, -13.0, -9.0, -8.4, -4.3, -2.4, -2.2, -6.1, -4.5],
            gains_20: [
                -10.6, -10.5, -10.8, -12.0, -13.0, -11.5, -9.0, -7.4, -8.4, -6.6,
                -4.3, -0.3, -2.4, -3.5, -2.2, -4.8, -6.1, -6.3, -4.5, -2.3,
            ],
        },
        Preset {
            name: "Razor Kraken Tournament Edition".to_string(),
            description: "AutoEq Harman Target (Rtings HMS II.3) 20-band calibrated profile".to_string(),
            preamp_db: -7.0,
            is_20_band: true,
            gains_10: [-0.3, -7.2, -11.7, -10.5, -12.1, -3.2, -0.3, -0.3, -9.4, -6.1],
            gains_20: [
                -0.3, -2.0, -7.2, -10.5, -11.7, -11.7, -10.5, -10.8, -12.1, -7.1,
                -3.2, -0.3, -0.3, -0.3, -0.3, -6.5, -9.4, -8.1, -6.1, -3.8,
            ],
        },
    ]
}

#[derive(Debug, Clone)]
pub struct AudioEqState {
    pub active_preset_idx: usize,
    pub presets: Vec<Preset>,
    pub is_20_band: bool,
    pub is_bypassed: bool,
    pub preamp_db: f32,
    pub gains_10: [f32; 10],
    pub gains_20: [f32; 20],
    pub node_id: Option<u32>,
    pub status_message: String,
}

impl Default for AudioEqState {
    fn default() -> Self {
        let mut presets = builtin_presets();
        load_saved_profiles(&mut presets);

        let (saved_preset, saved_bypassed, saved_gains_10) = load_eq_toml();
        let active_idx = presets
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(&saved_preset))
            .unwrap_or(0);

        let initial_preset = &presets[active_idx];
        let is_20_band = initial_preset.is_20_band;
        let preamp_db = initial_preset.preamp_db;
        let gains_10 = if initial_preset.name == "Flat" && saved_gains_10.iter().any(|&g| g.abs() > 0.01) {
            saved_gains_10
        } else {
            initial_preset.gains_10
        };
        let gains_20 = initial_preset.gains_20;

        let node_id = find_equalizer_node_id();
        let status = if let Some(id) = node_id {
            format!("PipeWire equalizer sink active (node #{id})")
        } else {
            "PipeWire equalizer node offline or unmapped".to_string()
        };

        let mut state = Self {
            active_preset_idx: active_idx,
            presets,
            is_20_band,
            is_bypassed: saved_bypassed,
            preamp_db,
            gains_10,
            gains_20,
            node_id,
            status_message: status,
        };

        state.apply_dsp();
        state
    }
}

pub static STATE: LazyLock<Mutex<AudioEqState>> = LazyLock::new(|| Mutex::new(AudioEqState::default()));

pub fn interpolate_gains(src_freqs: &[f32], src_gains: &[f32], target_freqs: &[f32]) -> Vec<f32> {
    if src_freqs.is_empty() || src_gains.is_empty() {
        return vec![0.0; target_freqs.len()];
    }
    if src_freqs.len() == 1 || src_gains.len() == 1 {
        return vec![src_gains[0]; target_freqs.len()];
    }

    let src_log: Vec<f32> = src_freqs.iter().map(|f| f.log10()).collect();

    target_freqs
        .iter()
        .map(|&tf| {
            let t_log = tf.log10();
            if t_log <= src_log[0] {
                return src_gains[0];
            }
            if t_log >= src_log[src_log.len() - 1] {
                return src_gains[src_log.len() - 1];
            }

            for i in 0..(src_log.len() - 1) {
                if t_log >= src_log[i] && t_log <= src_log[i + 1] {
                    let factor = (t_log - src_log[i]) / (src_log[i + 1] - src_log[i]);
                    let interpolated = src_gains[i] + factor * (src_gains[i + 1] - src_gains[i]);
                    return (interpolated * 10.0).round() / 10.0;
                }
            }

            0.0
        })
        .collect()
}

fn load_saved_profiles(presets: &mut Vec<Preset>) {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/rcrout"));
    let paths = [
        format!("{home}/.config/amane/profiles"),
        format!("{home}/.config/rusteq/profiles"),
    ];

    for dir in paths {
        let p = Path::new(&dir);
        if let Ok(entries) = fs::read_dir(p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                            let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if name.is_empty() || presets.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
                                continue;
                            }
                            let desc = val.get("description").and_then(|v| v.as_str()).unwrap_or("Imported AutoEq profile").to_string();
                            let preamp = val.get("preamp_db").and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(0.0);
                            let is_20 = val.get("band_mode").and_then(|v| v.as_str()).unwrap_or("20") == "20";

                            let mut g10 = [0.0f32; 10];
                            if let Some(arr) = val.get("gains_10").and_then(|v| v.as_array()) {
                                for (i, item) in arr.iter().take(10).enumerate() {
                                    g10[i] = item.as_f64().map(|v| v as f32).unwrap_or(0.0);
                                }
                            }

                            let mut g20 = [0.0f32; 20];
                            if let Some(arr) = val.get("gains_20").and_then(|v| v.as_array()) {
                                for (i, item) in arr.iter().take(20).enumerate() {
                                    g20[i] = item.as_f64().map(|v| v as f32).unwrap_or(0.0);
                                }
                            }

                            presets.push(Preset {
                                name,
                                description: desc,
                                preamp_db: preamp,
                                is_20_band: is_20,
                                gains_10: g10,
                                gains_20: g20,
                            });
                        }
                    }
                }
            }
        }
    }
}

fn load_eq_toml() -> (String, bool, [f32; 10]) {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/rcrout"));
    let path = format!("{home}/.config/amane/eq.toml");
    let Ok(content) = fs::read_to_string(&path) else {
        return (String::from("Flat"), false, [0.0; 10]);
    };

    let mut preset = String::from("Flat");
    let mut bypassed = false;
    let mut gains = [0.0f32; 10];
    let mut in_g10 = false;
    let mut idx = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("active_preset") {
            if let Some((_, val)) = trimmed.split_once('=') {
                preset = val.trim().trim_matches('"').to_string();
            }
        } else if trimmed.starts_with("is_bypassed") {
            if let Some((_, val)) = trimmed.split_once('=') {
                bypassed = val.trim() == "true";
            }
        } else if trimmed.starts_with("gains_10 = [") || trimmed == "gains_10 = [" {
            in_g10 = true;
            idx = 0;
        } else if in_g10 {
            if trimmed.starts_with(']') {
                in_g10 = false;
            } else if idx < 10 {
                let num_str = trimmed.trim_end_matches(',');
                if let Ok(v) = num_str.parse::<f32>() {
                    gains[idx] = v;
                    idx += 1;
                }
            }
        }
    }

    (preset, bypassed, gains)
}

fn save_eq_toml(state: &AudioEqState) {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/rcrout"));
    let path = format!("{home}/.config/amane/eq.toml");
    let active_name = state.presets.get(state.active_preset_idx).map(|p| p.name.as_str()).unwrap_or("Custom");

    let mut toml = String::new();
    toml.push_str(&format!("active_preset = \"{}\"\n", active_name));
    toml.push_str(&format!("is_bypassed = {}\n", state.is_bypassed));
    toml.push_str(&format!("band_mode = \"{}\"\n", if state.is_20_band { "20" } else { "10" }));
    toml.push_str(&format!("preamp_db = {:.2}\n", state.preamp_db));
    toml.push_str("gains_10 = [\n");
    for g in &state.gains_10 {
        toml.push_str(&format!("    {:.1},\n", g));
    }
    toml.push_str("]\n");

    let _ = fs::write(&path, toml);
}

fn find_equalizer_node_id() -> Option<u32> {
    let output = Command::new("pw-cli")
        .args(["list-objects", "Node"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);

    let mut current_id = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("id ") {
            if let Some((_, rest)) = trimmed.split_once("id ") {
                if let Some((id_str, _)) = rest.split_once(',') {
                    current_id = id_str.trim().parse::<u32>().ok();
                }
            }
        } else if (trimmed.starts_with("node.name = \"Equalizer\"") && !trimmed.contains("Equalizer-output"))
            || trimmed == "node.name = \"RustEQ\""
        {
            return current_id;
        }
    }
    None
}

impl AudioEqState {
    pub fn apply_dsp(&mut self) {
        if self.node_id.is_none() {
            self.node_id = find_equalizer_node_id();
        }

        save_eq_toml(self);

        let Some(nid) = self.node_id else {
            return;
        };

        let effective_preamp = if self.is_bypassed { 0.0 } else { self.preamp_db };

        let mut params = format!("{{ params = [ \"eq_preamp:Gain\" {:.2} ", effective_preamp);

        if self.is_20_band {
            for i in 0..20 {
                let g = if self.is_bypassed { 0.0 } else { self.gains_20[i] };
                params.push_str(&format!("\"eq_band_{}:Gain\" {:.2} ", i + 1, g));
            }
        } else {
            for (i, &g) in self.gains_10.iter().enumerate() {
                let b = i * 2 + 1;
                let gain = if self.is_bypassed { 0.0 } else { g };
                params.push_str(&format!("\"eq_band_{b}:Gain\" {gain:.2} "));
                if b < 19 {
                    let next_g = if self.is_bypassed { 0.0 } else { self.gains_10[i + 1] };
                    let mid_g = (gain + next_g) / 2.0;
                    let mid_b = b + 1;
                    params.push_str(&format!("\"eq_band_{mid_b}:Gain\" {mid_g:.2} "));
                } else {
                    params.push_str(&format!("\"eq_band_20:Gain\" {gain:.2} "));
                }
            }
        }
        params.push_str("] }");

        let _ = Command::new("pw-cli")
            .args(["s", &nid.to_string(), "Props", &params])
            .status();
    }
}

pub fn cycle_preset(delta: isize) {
    let mut state = STATE.lock().unwrap();
    if state.presets.is_empty() {
        return;
    }
    let count = state.presets.len() as isize;
    let next = (state.active_preset_idx as isize + delta).rem_euclid(count) as usize;
    state.active_preset_idx = next;

    let p = state.presets[next].clone();
    state.preamp_db = p.preamp_db;
    state.is_20_band = p.is_20_band;
    state.gains_10 = p.gains_10;
    state.gains_20 = p.gains_20;
    state.status_message = format!("Loaded preset '{}' ({})", p.name, p.description);

    state.apply_dsp();
}

pub fn toggle_mode() {
    let mut state = STATE.lock().unwrap();
    state.is_20_band = !state.is_20_band;

    if state.is_20_band {
        let interpolated = interpolate_gains(&ISO_10_FREQUENCIES, &state.gains_10, &ISO_20_FREQUENCIES);
        for (i, &v) in interpolated.iter().take(20).enumerate() {
            state.gains_20[i] = v;
        }
    } else {
        let interpolated = interpolate_gains(&ISO_20_FREQUENCIES, &state.gains_20, &ISO_10_FREQUENCIES);
        for (i, &v) in interpolated.iter().take(10).enumerate() {
            state.gains_10[i] = v;
        }
    }

    state.status_message = format!("Switched to {}-Band EQ mode", if state.is_20_band { 20 } else { 10 });
    state.apply_dsp();
}

pub fn toggle_bypass() {
    let mut state = STATE.lock().unwrap();
    state.is_bypassed = !state.is_bypassed;
    state.status_message = if state.is_bypassed {
        "Equalizer is now bypassed (flat hardware output)".to_string()
    } else {
        "Equalizer active and filtering audio".to_string()
    };
    state.apply_dsp();
}

pub fn adjust_preamp(delta: f32) {
    let mut state = STATE.lock().unwrap();
    state.preamp_db = (state.preamp_db + delta).clamp(-24.0, 12.0);
    state.apply_dsp();
}

pub fn reset_flat() {
    let mut state = STATE.lock().unwrap();
    state.preamp_db = 0.0;
    state.gains_10 = [0.0; 10];
    state.gains_20 = [0.0; 20];
    state.status_message = "Equalizer reset to flat (0.0 dB all bands, 0.0 dB preamp)".to_string();
    state.apply_dsp();
}

pub fn set_band_gain(idx: usize, val: f32) {
    let mut state = STATE.lock().unwrap();
    let clamped = (val * 10.0).round() / 10.0;
    let clamped = clamped.clamp(-24.0, 12.0);

    if state.is_20_band {
        if idx < 20 {
            state.gains_20[idx] = clamped;
        }
    } else if idx < 10 {
        state.gains_10[idx] = clamped;
    }
    state.apply_dsp();
}

pub fn adjust_band_gain(idx: usize, delta: f32) {
    let mut state = STATE.lock().unwrap();
    if state.is_20_band {
        if idx < 20 {
            let next = (state.gains_20[idx] + delta).clamp(-24.0, 12.0);
            state.gains_20[idx] = (next * 10.0).round() / 10.0;
        }
    } else if idx < 10 {
        let next = (state.gains_10[idx] + delta).clamp(-24.0, 12.0);
        state.gains_10[idx] = (next * 10.0).round() / 10.0;
    }
    state.apply_dsp();
}

pub fn export_profile() {
    let state = STATE.lock().unwrap().clone();
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/rcrout"));
    let dir = format!("{home}/.config/amane/profiles");
    let _ = fs::create_dir_all(&dir);

    let active_name = state.presets.get(state.active_preset_idx).map(|p| p.name.as_str()).unwrap_or("Custom Profile");
    let filename = format!("{dir}/{active_name}.json");

    let val = serde_json::json!({
        "name": active_name,
        "description": "User exported profile from Amane Audio Equalizer",
        "preamp_db": state.preamp_db,
        "band_mode": if state.is_20_band { "20" } else { "10" },
        "gains_10": state.gains_10,
        "gains_20": state.gains_20,
    });

    if let Ok(content) = serde_json::to_string_pretty(&val) {
        let _ = fs::write(&filename, content);
        if let Ok(mut state) = STATE.lock() {
            state.status_message = format!("Exported profile to {}", filename);
        }
    }
}

pub fn import_profile() {
    let mut state = STATE.lock().unwrap();
    let mut presets = builtin_presets();
    load_saved_profiles(&mut presets);
    let count = presets.len();
    state.presets = presets;
    state.status_message = format!("Refreshed available profiles ({} profiles loaded)", count);
}

pub fn build(page: &mut Page) {
    let state = STATE.lock().unwrap().clone();
    let theme = page.theme;
    let width = page.width;

    // 1. Header Card with Title and Status
    let header_widget = build_header(theme, width, &state.status_message, state.node_id.is_some());
    page.row(52.0, header_widget);
    page.end_group();

    // 2. Preset, Band Mode, and Bypass Controls Card
    let cur_preset = state.presets.get(state.active_preset_idx);
    let cur_name = cur_preset.map(|p| p.name.as_str()).unwrap_or("Flat");
    let cur_desc = cur_preset.map(|p| p.description.as_str()).unwrap_or("Equalizer profile");

    let preset_ctrl = build_preset_selector(theme, cur_name);
    let preset_row = row::view(
        theme,
        width,
        row::HEIGHT,
        "Active preset",
        cur_desc,
        preset_ctrl,
        280.0,
    );
    page.row(row::HEIGHT, preset_row);

    // Band Mode row (10 vs 20 Bands)
    let mode_btn_label = if state.is_20_band { "20 Bands (Precision)" } else { "10 Bands (Standard)" };
    let mode_btn = button::view(theme, mode_btn_label, Style::Plain, true, toggle_mode);
    let mode_row = row::view(
        theme,
        width,
        row::HEIGHT,
        "Equalizer resolution",
        if state.is_20_band { "20 ISO bands with 1/3-octave resolution" } else { "10 standard octave bands" },
        mode_btn,
        button::width(mode_btn_label),
    );
    page.row(row::HEIGHT, mode_row);

    // Bypass / Active row
    let active_btn_label = if state.is_bypassed { "Bypassed (Off)" } else { "Active (Filtering)" };
    let active_style = if state.is_bypassed { Style::Plain } else { Style::Primary };
    let active_btn = button::view(theme, active_btn_label, active_style, true, toggle_bypass);
    let active_row = row::view(
        theme,
        width,
        row::HEIGHT,
        "Filter chain processing",
        if state.is_bypassed { "DSP filter chain is currently bypassed" } else { "PipeWire filter chain actively shaping audio" },
        active_btn,
        button::width(active_btn_label),
    );
    page.row(row::HEIGHT, active_row);

    // Preamp row
    let preamp_ctrl = build_preamp_control(theme, state.preamp_db);
    let preamp_row = row::view(
        theme,
        width,
        row::HEIGHT,
        "Preamp gain",
        "Digital headroom attenuation (-24 dB to +12 dB)",
        preamp_ctrl,
        200.0,
    );
    page.row(row::HEIGHT, preamp_row);

    page.end_group();

    // 3. Graphical Console Equalizer Faders Card
    let faders_card = build_faders_card(theme, width, &state);
    page.row(220.0, faders_card);
    page.end_group();

    // 4. Action Buttons Group: Import, Export, Reset Flat
    let actions_row = build_actions_row(theme, width);
    page.row(row::HEIGHT, actions_row);
    page.end_group();
}

fn build_header(theme: &Theme, width: f32, status_msg: &str, connected: bool) -> Rectangle {
    let icon = Text::new("󰕾").size(16.0).font(fonts::NERD).tight().color(theme.accent);
    let title = Text::new("Parametric & Graphic Equalizer")
        .size(15.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let status_color = if connected { theme.accent } else { theme.secondary_text };
    let dot = Text::new(if connected { "●" } else { "○" })
        .size(9.0)
        .font(fonts::BODY)
        .color(status_color);
    let subtitle = Text::new(status_msg)
        .size(12.0)
        .font(fonts::BODY)
        .color(theme.muted_text)
        .elide();

    let text_col = Column::new(children![
        Row::new(children![icon, title]).gap(8.0).align(Center),
        Row::new(children![dot, subtitle]).gap(6.0).align(Center),
    ])
    .gap(2.0);

    Rectangle::new()
        .width(width)
        .height(52.0)
        .padding(Padding {
            top: 6.0,
            bottom: 6.0,
            left: 18.0,
            right: 18.0,
        })
        .align_child(Start, Center)
        .child(text_col)
}

fn build_preset_selector(theme: &Theme, preset_name: &str) -> Rectangle {
    let prev_btn = Rectangle::new()
        .width(28.0)
        .height(28.0)
        .radius(8.0)
        .fill(high_surface(theme))
        .cursor(Pointer)
        .on_click(|_| cycle_preset(-1))
        .align_child(Center, Center)
        .child(Text::new("󰅁").size(14.0).font(fonts::NERD).tight().color(theme.text));

    let next_btn = Rectangle::new()
        .width(28.0)
        .height(28.0)
        .radius(8.0)
        .fill(high_surface(theme))
        .cursor(Pointer)
        .on_click(|_| cycle_preset(1))
        .align_child(Center, Center)
        .child(Text::new("󰅂").size(14.0).font(fonts::NERD).tight().color(theme.text));

    let label_w = 280.0 - 56.0 - 16.0;
    let label = Text::new(preset_name)
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.accent)
        .elide();

    let center_box = Rectangle::new()
        .width(label_w)
        .height(28.0)
        .radius(8.0)
        .fill(high_surface(theme))
        .padding(Padding {
            top: 0.0,
            bottom: 0.0,
            left: 10.0,
            right: 10.0,
        })
        .align_child(Center, Center)
        .child(label);

    let row = Row::new(children![prev_btn, center_box, next_btn])
        .gap(8.0)
        .align(Center);

    Rectangle::new()
        .width(280.0)
        .height(row::CONTROL_HEIGHT)
        .align_child(Start, Center)
        .child(row)
}

fn build_preamp_control(theme: &Theme, preamp: f32) -> Rectangle {
    let minus_btn = Rectangle::new()
        .width(28.0)
        .height(28.0)
        .radius(8.0)
        .fill(high_surface(theme))
        .cursor(Pointer)
        .on_click(|_| adjust_preamp(-0.5))
        .align_child(Center, Center)
        .child(Text::new("−").size(14.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text));

    let plus_btn = Rectangle::new()
        .width(28.0)
        .height(28.0)
        .radius(8.0)
        .fill(high_surface(theme))
        .cursor(Pointer)
        .on_click(|_| adjust_preamp(0.5))
        .align_child(Center, Center)
        .child(Text::new("+").size(14.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text));

    let label_str = format!("{preamp:+.1} dB");
    let label = Text::new(&label_str)
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(if preamp.abs() > 0.01 { theme.accent } else { theme.text });

    let center_box = Rectangle::new()
        .width(80.0)
        .height(28.0)
        .radius(8.0)
        .fill(high_surface(theme))
        .align_child(Center, Center)
        .child(label);

    let row = Row::new(children![minus_btn, center_box, plus_btn])
        .gap(8.0)
        .align(Center);

    Rectangle::new()
        .width(200.0)
        .height(row::CONTROL_HEIGHT)
        .align_child(Start, Center)
        .child(row)
}

fn build_faders_card(theme: &Theme, width: f32, state: &AudioEqState) -> Rectangle {
    let band_count = if state.is_20_band { 20 } else { 10 };
    let fader_w = ((width - 24.0) / band_count as f32).floor();
    let fader_h = 220.0 - 16.0;

    let mut cols: Vec<Box<dyn Widget>> = Vec::new();

    for i in 0..band_count {
        let (freq_label, gain) = if state.is_20_band {
            (ISO_20_LABELS[i], state.gains_20[i])
        } else {
            (ISO_10_LABELS[i], state.gains_10[i])
        };

        cols.push(Box::new(fader_widget(
            theme,
            i,
            freq_label,
            gain,
            state.is_bypassed,
            fader_w,
            fader_h,
        )));
    }

    let faders_row = Row::new(cols).gap(0.0).align(Center);

    Rectangle::new()
        .width(width)
        .height(220.0)
        .radius(16.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 8.0,
            bottom: 8.0,
            left: 12.0,
            right: 12.0,
        })
        .align_child(Center, Center)
        .child(faders_row)
}

fn fader_widget(
    theme: &Theme,
    idx: usize,
    freq_label: &str,
    gain: f32,
    is_bypassed: bool,
    width: f32,
    height: f32,
) -> Column {
    let gain_text = if gain > 0.0 {
        format!("+{gain:.1}")
    } else {
        format!("{gain:.1}")
    };

    let gain_color = if is_bypassed {
        theme.muted_text
    } else if gain.abs() > 0.01 {
        theme.accent
    } else {
        theme.muted_text
    };

    let top_label = Rectangle::new()
        .width(width)
        .height(14.0)
        .align_child(Center, Center)
        .child(
            Text::new(&gain_text)
                .size(9.5)
                .font(fonts::BODY)
                .weight(Weight::SemiBold)
                .color(gain_color),
        );

    let bottom_label = Rectangle::new()
        .width(width)
        .height(14.0)
        .align_child(Center, Center)
        .child(
            Text::new(freq_label)
                .size(9.5)
                .font(fonts::BODY)
                .color(theme.secondary_text),
        );

    let track_h = (height - 28.0 - 6.0).max(50.0);
    let slider = fader_track(theme, idx, gain, is_bypassed, width, track_h);

    Column::new(children![top_label, slider, bottom_label])
        .width(width)
        .align(Center)
        .gap(3.0)
}

fn fader_track(
    theme: &Theme,
    idx: usize,
    gain: f32,
    is_bypassed: bool,
    width: f32,
    height: f32,
) -> Stack {
    let track_w: f32 = 4.0;
    let track_x = (width - track_w) / 2.0;

    let handle_w: f32 = (width * 0.72).clamp(16.0, 24.0);
    let handle_h: f32 = 10.0;
    let handle_x = (width - handle_w) / 2.0;

    let travel = height - handle_h;

    // Norm maps -12..+12 dB (or clamped) to 0.0..1.0
    let norm = (gain.clamp(-12.0, 12.0) + 12.0) / 24.0;
    let handle_y = (1.0 - norm) * travel;

    let bg = Rectangle::new()
        .width(track_w)
        .height(height)
        .radius(track_w / 2.0)
        .fill(theme.border)
        .translate(track_x, 0.0);

    let zero_notch = Rectangle::new()
        .width(handle_w + 4.0)
        .height(1.5)
        .fill(theme::with_opacity(theme.secondary_text, 0.35))
        .translate((width - (handle_w + 4.0)) / 2.0, height / 2.0 - 0.75);

    let handle_color = if is_bypassed {
        theme.muted_text
    } else {
        theme.accent
    };

    let handle = Rectangle::new()
        .width(handle_w)
        .height(handle_h)
        .radius(handle_h / 2.0)
        .fill(handle_color)
        .translate(handle_x, handle_y);

    let area = Rectangle::new()
        .width(width)
        .height(height)
        .cursor(Pointer)
        .on_scroll(move |scroll: Scroll| {
            let delta = if scroll.y < 0.0 { 0.5 } else { -0.5 };
            adjust_band_gain(idx, delta);
        })
        .on_drag(move |point: Point| {
            let along = 1.0 - (point.y - handle_h / 2.0) / travel;
            let wanted_db = along.clamp(0.0, 1.0) * 24.0 - 12.0;
            set_band_gain(idx, wanted_db);
        })
        .on_click(move |button: Button| {
            if button == Button::Right {
                set_band_gain(idx, 0.0);
            }
        });

    Stack::new(children![bg, zero_notch, handle, area])
        .width(width)
        .height(height)
}

fn build_actions_row(theme: &Theme, width: f32) -> Rectangle {
    let import_btn = button::view(theme, "Import Profile...", Style::Plain, true, import_profile);
    let export_btn = button::view(theme, "Export Profile...", Style::Plain, true, export_profile);
    let reset_btn = button::view(theme, "Reset Flat", Style::Plain, true, reset_flat);

    let buttons = Row::new(children![import_btn, export_btn, reset_btn]).gap(10.0);

    row::view(
        theme,
        width,
        row::HEIGHT,
        "Equalizer profile management",
        "Import, export, or reset all bands to zero gain",
        buttons,
        button::width("Import Profile...") + button::width("Export Profile...") + button::width("Reset Flat") + 20.0,
    )
}
