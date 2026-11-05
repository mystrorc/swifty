use std::process::Command;

use amane::{
    Button, Center, Column, Padding, Point, Pointer, Rectangle, Row, Scroll, ScrollArea, Service,
    SpaceBetween, Start, Stack, Text, Weight, Widget, children,
};

use crate::fonts;
use crate::overlay::audio_control::{AudioDevice, AudioDevices};
use crate::theme::{self, Theme};

pub const PRESETS: [&str; 10] = [
    "Flat",
    "Sony WH-1000XM3",
    "Bass Boost",
    "Vocal Clarity",
    "Treble Boost",
    "Rock",
    "Electronic",
    "Classical",
    "Acoustic",
    "Razor Kraken Tournament Edition",
];

pub const PRESET_GAINS: [[f32; 10]; 10] = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],          // Flat
    [-10.6, -10.8, -13.0, -9.0, -8.4, -4.3, -2.4, -2.2, -6.1, -4.5], // Sony WH-1000XM3
    [7.0, 6.0, 5.0, 3.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],          // Bass Boost
    [-2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0, -1.0],      // Vocal Clarity
    [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 3.0, 5.0, 6.0, 7.0],          // Treble Boost
    [5.0, 4.0, 2.0, -1.0, -2.0, -1.0, 2.0, 4.0, 5.0, 6.0],       // Rock
    [6.0, 5.0, 3.0, 0.0, -2.0, 2.0, 1.0, 3.0, 5.0, 6.0],       // Electronic
    [4.0, 3.0, 2.0, 1.0, -1.0, -1.0, 0.0, 2.0, 3.0, 4.0],       // Classical
    [3.0, 2.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 2.0],         // Acoustic
    [-0.3, -7.2, -11.7, -10.5, -12.1, -3.2, -0.3, -0.3, -9.4, -6.1], // Razor Kraken Tournament Edition
];

pub const BANDS: [&str; 10] = [
    "31", "63", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
];

#[derive(Clone)]
pub struct EqData {
    pub active_preset: String,
    pub is_bypassed: bool,
    pub gains_10: [f32; 10],
    pub node_id: Option<u32>,
}

impl Default for EqData {
    fn default() -> Self {
        Self {
            active_preset: String::from("Flat"),
            is_bypassed: false,
            gains_10: [0.0; 10],
            node_id: None,
        }
    }
}

impl Service for EqData {
    fn new() -> Self {
        let mut data = Self::default();
        let (preset, bypassed, gains) = load_eq_state();
        data.active_preset = preset;
        data.is_bypassed = bypassed;
        data.gains_10 = gains;
        data.node_id = find_equalizer_node_id();

        if let Some(id) = data.node_id {
            if data.is_bypassed {
                apply_all_gains_to_node(id, &[0.0; 10]);
            } else {
                apply_all_gains_to_node(id, &data.gains_10);
            }
        }

        data
    }

    // only changes through user input
    fn listen() {}
}

fn eq_state_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/rcrout"));
    format!("{home}/.config/amane/eq.toml")
}

fn load_eq_state() -> (String, bool, [f32; 10]) {
    let path = eq_state_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
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

fn save_eq_state(preset: &str, bypassed: bool, gains: &[f32; 10]) {
    let path = eq_state_path();
    let mut toml = String::new();
    toml.push_str(&format!("active_preset = \"{}\"\n", preset));
    toml.push_str(&format!("is_bypassed = {}\n", bypassed));
    toml.push_str("gains_10 = [\n");
    for g in gains {
        toml.push_str(&format!("    {:.1},\n", g));
    }
    toml.push_str("]\n");

    let _ = std::fs::write(&path, toml);
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
        } else if trimmed.contains("node.name = \"Equalizer\"") {
            return current_id;
        }
    }
    current_id
}

fn apply_all_gains_to_node(node_id: u32, gains: &[f32; 10]) {
    let mut params = String::from("{ params = [ ");
    for (i, &g) in gains.iter().enumerate() {
        let b = i * 2 + 1;
        params.push_str(&format!("\"eq_band_{b}:Gain\" {g:.1} "));
        if b < 19 {
            let next_g = gains[i + 1];
            let mid_g = (g + next_g) / 2.0;
            let mid_b = b + 1;
            params.push_str(&format!("\"eq_band_{mid_b}:Gain\" {mid_g:.1} "));
        } else {
            params.push_str(&format!("\"eq_band_20:Gain\" {g:.1} "));
        }
    }
    params.push_str("] }");

    let _ = Command::new("pw-cli")
        .args(["s", &node_id.to_string(), "Props", &params])
        .status();
}

pub fn set_preset(preset: &str) {
    let preset_idx = PRESETS
        .iter()
        .position(|&p| {
            p.eq_ignore_ascii_case(preset)
                || (preset.eq_ignore_ascii_case("Sony XM3") && p.contains("WH-1000XM3"))
                || (preset.eq_ignore_ascii_case("XM3") && p.contains("WH-1000XM3"))
        })
        .unwrap_or(0);

    let preset_name = PRESETS[preset_idx];
    let gains = PRESET_GAINS[preset_idx];

    let (node_id, bypassed) = {
        let mut data = EqData::write();
        data.active_preset = preset_name.to_string();
        data.gains_10 = gains;
        (data.node_id, data.is_bypassed)
    };

    save_eq_state(preset_name, bypassed, &gains);

    if let Some(id) = node_id {
        if !bypassed {
            apply_all_gains_to_node(id, &gains);
        }
    }
}

pub fn cycle_preset(direction: i32) {
    let current = EqData::read().active_preset.clone();
    let idx = PRESETS
        .iter()
        .position(|&p| p.eq_ignore_ascii_case(&current))
        .unwrap_or(0);

    let next_idx = if direction > 0 {
        (idx + 1) % PRESETS.len()
    } else {
        (idx + PRESETS.len() - 1) % PRESETS.len()
    };

    set_preset(PRESETS[next_idx]);
}

pub fn toggle_bypass() {
    let (new_bypassed, node_id, gains, preset) = {
        let mut eq = EqData::write();
        eq.is_bypassed = !eq.is_bypassed;
        (eq.is_bypassed, eq.node_id, eq.gains_10, eq.active_preset.clone())
    };

    save_eq_state(&preset, new_bypassed, &gains);

    if let Some(id) = node_id {
        if new_bypassed {
            apply_all_gains_to_node(id, &[0.0; 10]);
        } else {
            apply_all_gains_to_node(id, &gains);
        }
    }
}

pub fn adjust_band_gain(band_idx: usize, delta: f32) {
    if band_idx >= 10 {
        return;
    }

    let (current_gain, node_id, is_bypassed) = {
        let mut eq = EqData::write();
        let g = (eq.gains_10[band_idx] + delta).clamp(-12.0, 12.0);
        eq.gains_10[band_idx] = (g * 2.0).round() / 2.0;
        eq.active_preset = String::from("Custom");
        save_eq_state("Custom", eq.is_bypassed, &eq.gains_10);
        (eq.gains_10[band_idx], eq.node_id, eq.is_bypassed)
    };

    if let Some(id) = node_id {
        if !is_bypassed {
            let eq_band = band_idx * 2 + 1;
            let param_str = format!("{{ params = [ \"eq_band_{eq_band}:Gain\" {current_gain:.1} ] }}");
            let _ = Command::new("pw-cli")
                .args(["s", &id.to_string(), "Props", &param_str])
                .status();
        }
    }
}

pub fn set_band_gain(band_idx: usize, gain: f32) {
    if band_idx >= 10 {
        return;
    }

    let clamped = (gain.clamp(-15.0, 15.0) * 2.0).round() / 2.0;
    let (node_id, is_bypassed) = {
        let mut eq = EqData::write();
        eq.gains_10[band_idx] = clamped;
        eq.active_preset = String::from("Custom");
        save_eq_state("Custom", eq.is_bypassed, &eq.gains_10);
        (eq.node_id, eq.is_bypassed)
    };

    if let Some(id) = node_id {
        if !is_bypassed {
            let eq_band = band_idx * 2 + 1;
            let param_str = format!("{{ params = [ \"eq_band_{eq_band}:Gain\" {clamped:.1} ] }}");
            let _ = Command::new("pw-cli")
                .args(["s", &id.to_string(), "Props", &param_str])
                .status();
        }
    }
}

pub fn bypass_button(theme: &Theme, bypassed: bool) -> Rectangle {


    let (icon, label, fill, text_color) = if bypassed {
        ("󰂱", "Bypassed", theme::with_opacity(theme.danger, 0.15), theme.danger)
    } else {
        ("󰂲", "Active", theme.selected_surface, theme.accent)
    };

    let icon_w = Text::new(icon).size(13.0).font(fonts::NERD).tight().color(text_color);
    let label_w = Text::new(label).size(11.0).font(fonts::BODY).weight(Weight::SemiBold).color(text_color);

    let row = Row::new(children![icon_w, label_w]).gap(5.0).align(Center);
    let width = label.chars().count() as f32 * 7.5 + 38.0;

    Rectangle::new()
        .width(width)
        .height(26.0)
        .radius(8.0)
        .fill(fill)
        .padding(Padding {
            top: 4.0,
            bottom: 4.0,
            left: 10.0,
            right: 10.0,
        })
        .cursor(Pointer)
        .on_click(|_| toggle_bypass())
        .align_child(Center, Center)
        .child(row)
}

pub fn sink_item(theme: &Theme, sink: &AudioDevice, width: f32) -> Rectangle {
    let id = sink.id;
    let is_default = sink.is_default;

    let fill = if is_default {
        theme.selected_surface
    } else {
        amane::Color::TRANSPARENT
    };

    let radio_icon = if is_default { "󰞌" } else { "󰝦" };
    let radio_color = if is_default { theme.accent } else { theme.muted_text };

    let name_lower = sink.name.to_lowercase();
    let dev_icon = if name_lower.contains("earbud")
        || name_lower.contains("headphone")
        || name_lower.contains("headset")
        || name_lower.contains("buds")
        || name_lower.contains("sony")
        || name_lower.contains("bose")
        || name_lower.contains("airpod")
        || name_lower.contains("wh-")
        || name_lower.contains("wf-")
    {
        "󰋋"
    } else if name_lower.contains("sound bar") || name_lower.contains("speaker") {
        "󰓃"
    } else {
        "󰕾"
    };

    let dev_icon_color = if is_default { theme.accent } else { theme.secondary_text };
    let text_color = if is_default { theme.text } else { theme.secondary_text };
    let weight = if is_default { Weight::SemiBold } else { Weight::Regular };

    let radio_w = Text::new(radio_icon).size(13.0).font(fonts::NERD).tight().color(radio_color);
    let dev_icon_w = Text::new(dev_icon).size(12.0).font(fonts::NERD).tight().color(dev_icon_color);

    let label_w = width - 16.0 - 13.0 - 12.0 - 12.0;
    let name_w = Text::new(&sink.name).size(11.0).font(fonts::BODY).weight(weight).color(text_color).elide();
    let name_box = Rectangle::new()
        .width(label_w.max(50.0))
        .height(16.0)
        .align_child(Start, Center)
        .child(name_w);

    let row = Row::new(children![radio_w, dev_icon_w, name_box]).gap(6.0).align(Center);

    Rectangle::new()
        .width(width)
        .height(28.0)
        .radius(7.0)
        .fill(fill)
        .padding(Padding {
            top: 2.0,
            bottom: 2.0,
            left: 8.0,
            right: 8.0,
        })
        .cursor(Pointer)
        .on_click(move |_| {
            AudioDevices::switch_sink(id);
        })
        .align_child(Start, Center)
        .child(row)
}


pub fn preset_selector(theme: &Theme, active_preset: &str, width: f32) -> Row {
    let prev_btn = Rectangle::new()
        .width(22.0)
        .height(22.0)
        .radius(6.0)
        .fill(theme.selected_surface)
        .cursor(Pointer)
        .on_click(|_| cycle_preset(-1))
        .align_child(Center, Center)
        .child(Text::new("󰁍").size(12.0).font(fonts::NERD).tight().color(theme.text));

    let next_btn = Rectangle::new()
        .width(22.0)
        .height(22.0)
        .radius(6.0)
        .fill(theme.selected_surface)
        .cursor(Pointer)
        .on_click(|_| cycle_preset(1))
        .align_child(Center, Center)
        .child(Text::new("󰁔").size(12.0).font(fonts::NERD).tight().color(theme.text));

    let preset_icon = Text::new("󰎄").size(11.0).font(fonts::NERD).tight().color(theme.accent);
    let preset_label = Text::new(active_preset).size(12.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text);

    let preset_center = Row::new(children![preset_icon, preset_label]).gap(5.0).align(Center);

    Row::new(children![
        Row::new(children![prev_btn, preset_center, next_btn]).gap(8.0).align(Center),
    ])
    .width(width)
    .align(Center)
}

pub fn bands_row(theme: &Theme, eq: &EqData, width: f32, height: f32) -> Row {
    let band_count = 10;
    let band_width = (width / band_count as f32).floor();

    let mut cols: Vec<Box<dyn Widget>> = Vec::new();

    for i in 0..10 {
        cols.push(Box::new(band_slider(theme, eq, i, band_width, height)));
    }

    Row::new(cols).width(width).align(Center)
}

fn band_slider(theme: &Theme, eq: &EqData, index: usize, width: f32, height: f32) -> Column {
    let gain = eq.gains_10[index];
    let is_bypassed = eq.is_bypassed;

    let gain_text = if gain > 0.0 {
        format!("+{:.0}", gain)
    } else {
        format!("{:.0}", gain)
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
        .height(13.0)
        .align_child(Center, Center)
        .child(Text::new(&gain_text).size(9.0).font(fonts::BODY).weight(Weight::SemiBold).color(gain_color));

    let bottom_label = Rectangle::new()
        .width(width)
        .height(13.0)
        .align_child(Center, Center)
        .child(Text::new(BANDS[index]).size(9.0).font(fonts::BODY).color(theme.muted_text));

    let track_height = (height - 26.0 - 4.0).max(40.0);
    let track_widget = slider_track(theme, gain, is_bypassed, index, width, track_height);

    Column::new(children![top_label, track_widget, bottom_label])
        .width(width)
        .align(Center)
        .gap(2.0)
}

fn slider_track(
    theme: &Theme,
    gain: f32,
    is_bypassed: bool,
    index: usize,
    width: f32,
    height: f32,
) -> Stack {
    let track_w: f32 = 4.0;
    let track_x = (width - track_w) / 2.0;

    let handle_w: f32 = 18.0;
    let handle_h: f32 = 10.0;
    let handle_x = (width - handle_w) / 2.0;

    let travel = height - handle_h;

    let norm = (gain.clamp(-15.0, 15.0) + 15.0) / 30.0;
    let handle_y = (1.0 - norm) * travel;

    let bg = Rectangle::new()
        .width(track_w)
        .height(height)
        .radius(track_w / 2.0)
        .fill(theme.border)
        .translate(track_x, 0.0);

    let zero_notch = Rectangle::new()
        .width(10.0)
        .height(1.5)
        .fill(theme::with_opacity(theme.secondary_text, 0.4))
        .translate((width - 10.0) / 2.0, height / 2.0 - 0.75);

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
            let delta = if scroll.y < 0.0 { 1.0 } else { -1.0 };
            adjust_band_gain(index, delta);
        })
        .on_drag(move |point: Point| {
            let along = 1.0 - (point.y - handle_h / 2.0) / travel;
            let wanted_db = along.clamp(0.0, 1.0) * 30.0 - 15.0;
            set_band_gain(index, wanted_db);
        })
        .on_click(move |button: Button| {
            if button == Button::Right {
                set_band_gain(index, 0.0);
            }
        });

    Stack::new(children![bg, zero_notch, handle, area])
        .width(width)
        .height(height)
}
