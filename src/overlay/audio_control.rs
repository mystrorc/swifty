use std::process::Command;

use amane::{
    Center, Column, Padding, Pointer, Rectangle, Row, ScrollArea, Service,
    SpaceBetween, Start, Text, Weight, Widget, children,
};

use super::{Overlay, PanelView, Region};
use crate::fonts;
use crate::liquid::{self, Blob};
use crate::theme::Theme;

const WIDTH: f32 = 360.0;
const HEIGHT: f32 = 410.0;
const RADIUS: f32 = 28.0;

const EDGE_OVERLAP: f32 = 40.0;
const HIDDEN_MARGIN: f32 = 32.0;

const PADDING: Padding = Padding {
    top: EDGE_OVERLAP + 16.0,
    right: 18.0,
    bottom: 18.0,
    left: 18.0,
};

const INNER_WIDTH: f32 = WIDTH - PADDING.left - PADDING.right;
const GAP: f32 = 12.0;
const CARD_PADDING: f32 = 14.0;
const CARD_INNER_WIDTH: f32 = INNER_WIDTH - CARD_PADDING * 2.0;

#[derive(Clone, Default, PartialEq)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Default)]
pub struct AudioDevices {
    pub sinks: Vec<AudioDevice>,
    pub sources: Vec<AudioDevice>,
}

impl Service for AudioDevices {
    fn new() -> Self {
        let mut devices = Self::default();
        devices.update();
        devices
    }

    fn interval() -> std::time::Duration {
        std::time::Duration::from_secs(2)
    }

    fn update(&mut self) -> bool {
        let (sinks, sources) = query_devices();
        if self.sinks != sinks || self.sources != sources {
            self.sinks = sinks;
            self.sources = sources;
            true
        } else {
            false
        }
    }
}

impl AudioDevices {
    pub fn select_sink(id: u32) {
        let mut devices = Self::write();
        for sink in &mut devices.sinks {
            sink.is_default = sink.id == id;
        }
    }

    pub fn switch_sink(id: u32) {
        Self::select_sink(id);

        std::thread::spawn(move || {
            let Ok(out) = Command::new("wpctl").args(["inspect", &id.to_string()]).output() else {
                return;
            };
            let text = String::from_utf8_lossy(&out.stdout);
            let mut target_node = None;
            for line in text.lines() {
                if line.contains("node.name =") {
                    if let Some((_, val)) = line.split_once('=') {
                        target_node = Some(val.trim().trim_matches('"').to_string());
                        break;
                    }
                }
            }

            let Some(target) = target_node else {
                let _ = Command::new("wpctl").args(["set-default", &id.to_string()]).status();
                return;
            };

            // Unlink previous Equalizer-output links
            if let Ok(lout) = Command::new("pw-link").arg("-l").output() {
                let ltext = String::from_utf8_lossy(&lout.stdout);
                let mut in_eq_fl = false;
                let mut in_eq_fr = false;
                for line in ltext.lines() {
                    if line.starts_with("Equalizer-output:output_FL") {
                        in_eq_fl = true;
                        in_eq_fr = false;
                    } else if line.starts_with("Equalizer-output:output_FR") {
                        in_eq_fr = true;
                        in_eq_fl = false;
                    } else if !line.starts_with("  ") {
                        in_eq_fl = false;
                        in_eq_fr = false;
                    } else if in_eq_fl && line.contains("|->") {
                        if let Some((_, port)) = line.split_once("|->") {
                            let p = port.trim();
                            if !p.is_empty() {
                                let _ = Command::new("pw-link").args(["-d", "Equalizer-output:output_FL", p]).status();
                            }
                        }
                    } else if in_eq_fr && line.contains("|->") {
                        if let Some((_, port)) = line.split_once("|->") {
                            let p = port.trim();
                            if !p.is_empty() {
                                let _ = Command::new("pw-link").args(["-d", "Equalizer-output:output_FR", p]).status();
                            }
                        }
                    }
                }
            }

            // Link Equalizer-output to target device
            let fl_port = format!("{}:playback_FL", target);
            let fr_port = format!("{}:playback_FR", target);
            let _ = Command::new("pw-link").args(["Equalizer-output:output_FL", &fl_port]).status();
            let _ = Command::new("pw-link").args(["Equalizer-output:output_FR", &fr_port]).status();

            // Set default sink to Equalizer
            let _ = Command::new("pactl").args(["set-default-sink", "Equalizer"]).status();

            // Move running streams to Equalizer
            let _ = Command::new("sh")
                .arg("-c")
                .arg(r#"pactl list sink-inputs short | awk '{print $1}' | while read -r input_id; do pactl move-sink-input $input_id Equalizer >/dev/null 2>&1 || true; done"#)
                .status();
        });
    }
}

pub fn view(overlay: &Overlay, theme: &Theme, screen: Region) -> Option<PanelView> {
    let progress = overlay.audio_control.progress.value();

    if !overlay.audio_control.shown && progress <= 0.001 {
        return None;
    }

    let width = WIDTH.min(screen.width - 24.0);
    let x = (screen.width - width) / 2.0;

    let hidden = -(HEIGHT + HIDDEN_MARGIN);
    let y = -EDGE_OVERLAP + hidden * (1.0 - progress);

    let blob = Blob::new(x, y, width, HEIGHT)
        .radius(RADIUS)
        .child(content(theme, width));

    let input = Region {
        x,
        y,
        width,
        height: HEIGHT,
    };

    let reach = Region {
        x: x - liquid::CONNECTION,
        y: 0.0,
        width: width + liquid::CONNECTION * 2.0,
        height: -EDGE_OVERLAP + HEIGHT + liquid::CONNECTION,
    };

    Some(PanelView { blob, input, reach })
}

fn content(theme: &Theme, width: f32) -> Rectangle {
    let devices = AudioDevices::read();

    let column = Column::new(children![
        header(theme),
        devices_card(theme, "Output Devices", "󰓃", &devices.sinks, 150.0, "audio_ctrl_sinks"),
        devices_card(theme, "Input Devices", "󰍬", &devices.sources, 140.0, "audio_ctrl_sources"),
    ])
    .gap(GAP)
    .width(INNER_WIDTH);

    Rectangle::new()
        .width(width)
        .height(HEIGHT)
        .padding(PADDING)
        .on_hover(Overlay::hover_audio_control)
        .child(column)
}

fn header(theme: &Theme) -> Row {
    let icon = Text::new("󰕾")
        .size(20.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.accent);

    let title = Text::new("Audio Control")
        .size(15.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let left = Row::new(children![icon, title]).gap(8.0).align(Center);

    let mixer_icon = Text::new("󰓃")
        .size(13.0)
        .font(fonts::NERD)
        .color(theme.secondary_text);

    let mixer_text = Text::new("Mixer")
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::Medium)
        .color(theme.secondary_text);

    let mixer_button = Rectangle::new()
        .width(62.0)
        .height(24.0)
        .radius(10.0)
        .fill(theme.surface)
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_click(|_| {
            let _ = Command::new("ghostty").args(["-e", "wiremix"]).spawn();
        })
        .child(Row::new(children![mixer_icon, mixer_text]).gap(4.0).align(Center));

    Row::new(children![left, mixer_button])
        .width(INNER_WIDTH)
        .justify(SpaceBetween)
        .align(Center)
}

fn devices_card(
    theme: &Theme,
    title: &str,
    icon: &str,
    devices: &[AudioDevice],
    height: f32,
    scroll_key: &'static str,
) -> Rectangle {
    let header_icon = Text::new(icon)
        .size(14.0)
        .font(fonts::NERD)
        .color(theme.secondary_text);

    let count_text = format!("{} ({})", title, devices.len());
    let header_title = Text::new(&count_text)
        .size(12.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.secondary_text);

    let header_row = Row::new(children![header_icon, header_title])
        .gap(6.0)
        .align(Center);

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();

    if devices.is_empty() {
        let none_text = Text::new("No devices detected")
            .size(12.0)
            .font(fonts::BODY)
            .color(theme.secondary_text);
        rows.push(Box::new(none_text));
    } else {
        for dev in devices {
            rows.push(Box::new(device_row(theme, dev.clone())));
        }
    }

    let scroll_height = (height - CARD_PADDING * 2.0 - 18.0 - 6.0).max(40.0);
    let scroll_area = ScrollArea::new(scroll_key, Column::new(rows).gap(6.0).width(CARD_INNER_WIDTH))
        .width(CARD_INNER_WIDTH)
        .height(scroll_height);

    let card_content = Column::new(children![header_row, scroll_area]).gap(6.0).width(CARD_INNER_WIDTH);

    Rectangle::new()
        .width(INNER_WIDTH)
        .height(height)
        .radius(16.0)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .child(card_content)
}

fn device_row(theme: &Theme, dev: AudioDevice) -> Rectangle {
    let (bullet_char, bullet_color, text_color, weight) = if dev.is_default {
        ("󰞌", theme.accent, theme.text, Weight::SemiBold)
    } else {
        ("󰝦", theme.secondary_text, theme.secondary_text, Weight::Regular)
    };

    let bullet = Text::new(bullet_char)
        .size(13.0)
        .font(fonts::NERD)
        .color(bullet_color);

    let name = Text::new(&dev.name)
        .size(12.0)
        .font(fonts::BODY)
        .weight(weight)
        .color(text_color)
        .elide();

    let name_box = Rectangle::new()
        .width(CARD_INNER_WIDTH - 24.0)
        .height(16.0)
        .align_child(Start, Center)
        .child(name);

    let row = Row::new(children![bullet, name_box])
        .gap(8.0)
        .align(Center);

    let id = dev.id;

    Rectangle::new()
        .width(CARD_INNER_WIDTH)
        .height(20.0)
        .align_child(Start, Center)
        .cursor(Pointer)
        .on_click(move |_| {
            AudioDevices::switch_sink(id);
        })
        .child(row)
}

fn query_devices() -> (Vec<AudioDevice>, Vec<AudioDevice>) {
    let Ok(output) = Command::new("wpctl").arg("status").output() else {
        return (Vec::new(), Vec::new());
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut sinks = Vec::new();
    let mut sources = Vec::new();
    let mut section = 0; // 1 = sinks, 2 = sources

    for line in text.lines() {
        if line.contains("Sinks:") {
            section = 1;
            continue;
        } else if line.contains("Sources:") {
            section = 2;
            continue;
        } else if line.contains("Filters:")
            || line.contains("Streams:")
            || line.contains("Video")
            || line.contains("Settings")
        {
            section = 0;
            continue;
        }

        if section == 0 {
            continue;
        }

        let cleaned = line.replace('│', "").replace("├─", "").replace("└─", "");
        let trimmed = cleaned.trim();
        if trimmed.is_empty() {
            continue;
        }

        let is_default = trimmed.contains('*');
        let unstarred = trimmed.replace('*', "");
        let unstarred = unstarred.trim();

        let Some((id_str, rest)) = unstarred.split_once('.') else {
            continue;
        };

        let Ok(id) = id_str.trim().parse::<u32>() else {
            continue;
        };

        let name_part = rest.split('[').next().unwrap_or("").trim();
        let name = name_part
            .replace(" Analog Stereo", "")
            .replace(" (alsa)", "");

        if section == 2 && name.contains("Webcam") {
            continue;
        }

        let dev = AudioDevice {
            id,
            name,
            is_default,
        };

        if section == 1 {
            sinks.push(dev);
        } else if section == 2 {
            sources.push(dev);
        }
    }

    if sinks.iter().all(|s| !s.is_default) {
        if let Some(target) = get_equalizer_target_node() {
            for sink in &mut sinks {
                if let Ok(out) = Command::new("wpctl").args(["inspect", &sink.id.to_string()]).output() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    if text.contains(&target) {
                        sink.is_default = true;
                        break;
                    }
                }
            }
        }
    }

    (sinks, sources)
}

pub fn ipc(arguments: &[String]) -> String {
    let monitor_arg = arguments.iter().find(|arg| !matches!(arg.as_str(), "toggle" | "show" | "hide")).map(String::as_str);
    if Overlay::read().control_center.shown && Overlay::read().control_tab == 1 {
        Overlay::toggle_control_center(monitor_arg);
    } else {
        Overlay::open_control_tab(1, monitor_arg);
    }
    String::from("ok")
}

fn get_equalizer_target_node() -> Option<String> {
    let out = Command::new("pw-link").arg("-l").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut in_fl = false;
    for line in text.lines() {
        if line.starts_with("Equalizer-output:output_FL") {
            in_fl = true;
        } else if !line.starts_with("  ") {
            in_fl = false;
        } else if in_fl && line.contains("|->") {
            if let Some((_, port)) = line.split_once("|->") {
                let p = port.trim();
                let node = p.split(':').next()?;
                return Some(node.to_string());
            }
        }
    }
    None
}
