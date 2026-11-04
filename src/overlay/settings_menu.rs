use std::fs;
use std::path::PathBuf;
use std::process::Command;

use amane::{
    Bluetooth, Brightness, Center, Column, Full, Network, Padding, Pointer,
    Rectangle, Row, ScrollArea, Service, SpaceBetween, Stack, Start, Text, Weight, Widget,
    children,
};

use super::{Overlay, PanelView, Region};
use crate::bar::pill;
use crate::fonts;
use crate::settings::Settings;
use crate::theme::Theme;

// Legacy standalone pulldown view (kept for API compatibility; always returns None)
pub fn view(_overlay: &Overlay, _theme: &Theme, _screen: Region) -> Option<PanelView> {
    None
}

// Embedded view rendered inside the middle pulldown (control_center) Settings tab
pub fn embedded_view(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Row {
    let gap = 14.0;
    let sidebar_w = 210.0;
    let content_w = width - sidebar_w - gap;

    let left = sidebar(overlay, theme, sidebar_w, height);
    let right = right_panel(overlay, theme, content_w, height);

    Row::new(children![left, right])
        .width(width)
        .height(height)
        .gap(gap)
        .align(Center)
}

fn sidebar(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = width - 24.0;

    let icon = Rectangle::new()
        .width(32.0)
        .height(32.0)
        .radius(9.0)
        .fill(theme.accent)
        .align_child(Center, Center)
        .child(
            Text::new("\u{f0493}")
                .size(16.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.on_accent),
        );

    let title_text = Text::new("Settings")
        .size(15.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let subtitle_text = Text::new("Preferences")
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::Light)
        .color(theme.secondary_text);

    let title_col = Column::new(children![title_text, subtitle_text]).gap(1.0);

    let header = Row::new(children![icon, title_col])
        .gap(10.0)
        .align(Center);

    let nav_items = [
        ("󰍹 Monitors", 0),
        ("󰖩 Network", 1),
        ("󰂯 Bluetooth", 2),
        ("󰏘 Appearance", 3),
        ("󰍹 Status Bar", 4),
        ("󰍉 App Launcher", 5),
        ("󰌌 Input & Touchpad", 6),
        ("󱒓 Integrations", 7),
    ];

    let mut buttons: Vec<Box<dyn Widget>> = Vec::new();
    for (label, idx) in nav_items {
        let active = overlay.settings_tab == idx;
        let fill = if active {
            theme.accent
        } else {
            theme.hover_surface
        };
        let text_color = if active {
            theme.on_accent
        } else {
            theme.text
        };

        let btn = Rectangle::new()
            .width(inner_width)
            .height(35.0)
            .radius(10.0)
            .fill(fill)
            .cursor(Pointer)
            .padding(Padding {
                top: 0.0,
                right: 12.0,
                bottom: 0.0,
                left: 12.0,
            })
            .on_click(move |_| Overlay::set_settings_tab(idx))
            .align_child(Start, Center)
            .child(
                Text::new(label)
                    .size(12.0)
                    .font(fonts::BODY)
                    .weight(if active { Weight::SemiBold } else { Weight::Medium })
                    .color(text_color),
            );

        buttons.push(Box::new(btn));
    }

    let nav_col = Column::new(buttons).gap(4.0);

    let full_settings_btn = Rectangle::new()
        .width(inner_width)
        .height(34.0)
        .radius(10.0)
        .fill(theme.hover_surface)
        .cursor(Pointer)
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 0.0,
            left: 12.0,
        })
        .on_click(|_| {
            Overlay::write().control_center.hide();
            crate::settings::open();
        })
        .align_child(Center, Center)
        .child(
            Row::new(children![
                Text::new("Full Settings")
                    .size(12.0)
                    .font(fonts::BODY)
                    .weight(Weight::Medium)
                    .color(theme.text),
                Text::new(" 󰅂")
                    .size(12.0)
                    .font(fonts::NERD)
                    .color(theme.accent)
            ])
            .align(Center),
        );

    let top_part = Column::new(children![header, nav_col]).gap(12.0);

    let container = Column::new(children![top_part, full_settings_btn])
        .height(height - 28.0)
        .justify(SpaceBetween);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(18.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 14.0,
            right: 12.0,
            bottom: 14.0,
            left: 12.0,
        })
        .child(container)
}

fn right_panel(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = width - 32.0;

    let (cat_title, cat_subtitle) = match overlay.settings_tab {
        0 => ("Monitors", "Display outputs, enable switches, and brightness controls"),
        1 => ("Network", "Wireless connections and network adapters"),
        2 => ("Bluetooth", "Wireless devices, headphones, and peripherals"),
        3 => ("Appearance", "Interface colors, motion speed, and wallpaper effects"),
        4 => ("Status Bar", "Visibility of bar components and clock format"),
        5 => ("App Launcher", "Search behavior, visual hints, and command execution"),
        6 => ("Input & Touchpad", "Tap gestures, scrolling direction, and mouse options"),
        _ => ("Integrations", "Automatic color synchronization with desktop applications"),
    };

    let title_w = Text::new(cat_title)
        .size(16.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let subtitle_w = Text::new(cat_subtitle)
        .size(11.5)
        .font(fonts::BODY)
        .weight(Weight::Light)
        .color(theme.secondary_text);

    let header_col = Column::new(children![title_w, subtitle_w]).gap(2.0);

    let page_content = match overlay.settings_tab {
        0 => monitors_tab(theme, inner_width),
        1 => network_tab(overlay, theme, inner_width),
        2 => bluetooth_tab(overlay, theme, inner_width),
        3 => appearance_tab(theme, inner_width),
        4 => bar_tab(theme, inner_width),
        5 => launcher_tab(theme, inner_width),
        6 => input_tab(theme, inner_width),
        _ => integrations_tab(theme, inner_width),
    };

    let scroll_id = match overlay.settings_tab {
        0 => "settings_emb:monitors",
        1 => "settings_emb:network",
        2 => "settings_emb:bluetooth",
        3 => "settings_emb:appearance",
        4 => "settings_emb:bar",
        5 => "settings_emb:launcher",
        6 => "settings_emb:input",
        _ => "settings_emb:integrations",
    };

    let avail_h = (height - 32.0 - 36.0 - 12.0).max(100.0);

    let scroll_area = Rectangle::new()
        .width(inner_width)
        .height(avail_h)
        .child(ScrollArea::new(scroll_id, page_content));

    let content_col = Column::new(children![header_col, scroll_area]).gap(12.0);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(18.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 16.0,
            right: 16.0,
            bottom: 16.0,
            left: 16.0,
        })
        .child(content_col)
}

// ---------------- Monitors Card ----------------

#[derive(Clone, Debug)]
struct MonitorInfo {
    name: String,
    description: String,
    resolution: String,
    enabled: bool,
}

fn get_monitors() -> Vec<MonitorInfo> {
    let mut result = Vec::new();
    let Ok(output) = Command::new("niri").args(["msg", "--json", "outputs"]).output() else {
        return default_monitors();
    };
    let Ok(text) = std::str::from_utf8(&output.stdout) else {
        return default_monitors();
    };
    let Ok(json): Result<serde_json::Value, _> = serde_json::from_str(text) else {
        return default_monitors();
    };
    let Some(map) = json.as_object() else {
        return default_monitors();
    };

    for (name, item) in map {
        let make = item.get("make").and_then(|v| v.as_str()).unwrap_or("");
        let model = item.get("model").and_then(|v| v.as_str()).unwrap_or("");
        let desc = if !make.is_empty() || !model.is_empty() {
            format!("{make} {model}").trim().to_string()
        } else {
            name.clone()
        };

        let logical = item.get("logical").filter(|l| !l.is_null());
        let enabled = logical.is_some();

        let resolution = if let Some(loc) = logical {
            let w = loc.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
            let h = loc.get("height").and_then(|v| v.as_i64()).unwrap_or(0);
            let scale = loc.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0);
            format!("{w}×{h}  •  {scale:.1}× Scale")
        } else if let Some(modes) = item.get("modes").and_then(|v| v.as_array()) {
            if let Some(first) = modes.first() {
                let w = first.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
                let h = first.get("height").and_then(|v| v.as_i64()).unwrap_or(0);
                format!("{w}×{h}")
            } else {
                String::from("Unknown resolution")
            }
        } else {
            String::from("Output Disabled")
        };

        result.push(MonitorInfo {
            name: name.clone(),
            description: desc,
            resolution,
            enabled,
        });
    }

    if result.is_empty() {
        return default_monitors();
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

fn default_monitors() -> Vec<MonitorInfo> {
    vec![MonitorInfo {
        name: String::from("eDP-1"),
        description: String::from("Internal Display"),
        resolution: String::from("3200×1800  •  2.0× Scale"),
        enabled: true,
    }]
}

fn monitors_tab(theme: &Theme, width: f32) -> Column {
    let monitors = get_monitors();
    let total_active = monitors.iter().filter(|m| m.enabled).count();

    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    // 1. Global Blue Light Filter Card (Night Light & Warmth)
    items.push(Box::new(bluelight_filter_card(theme, width)));

    // 2. Connected Displays
    for mon in monitors {
        let name_clone = mon.name.clone();
        let is_enabled = mon.enabled;

        // Top row: Monitor Icon + Name & Info on left; Enable switch on right
        let icon_box = Rectangle::new()
            .width(36.0)
            .height(36.0)
            .radius(10.0)
            .fill(if is_enabled { theme.accent } else { theme.surface })
            .align_child(Center, Center)
            .child(
                Text::new("󰍹")
                    .size(18.0)
                    .font(fonts::NERD)
                    .tight()
                    .color(if is_enabled { theme.on_accent } else { theme.secondary_text }),
            );

        let title_row = Row::new(children![
            Text::new(&mon.name)
                .size(14.0)
                .font(fonts::BODY)
                .weight(Weight::SemiBold)
                .color(theme.text),
            Text::new(format!("({})", mon.description))
                .size(12.0)
                .font(fonts::BODY)
                .weight(Weight::Regular)
                .color(theme.secondary_text)
        ])
        .gap(8.0)
        .align(Center);

        let sub_info = Text::new(&mon.resolution)
            .size(11.0)
            .font(fonts::BODY)
            .weight(Weight::Light)
            .color(if is_enabled { theme.secondary_text } else { theme.danger });

        let text_col = Column::new(children![title_row, sub_info]).gap(2.0);

        let left_part = Row::new(children![icon_box, text_col])
            .gap(12.0)
            .align(Center);

        // Enable Switch
        let switch_pill = if is_enabled {
            Rectangle::new()
                .width(44.0)
                .height(24.0)
                .radius(Full)
                .fill(theme.accent)
                .cursor(Pointer)
                .align_child(amane::End, Center)
                .padding(Padding { top: 2.0, right: 3.0, bottom: 2.0, left: 3.0 })
                .child(
                    Rectangle::new()
                        .width(20.0)
                        .height(20.0)
                        .radius(Full)
                        .fill(theme.on_accent),
                )
        } else {
            Rectangle::new()
                .width(44.0)
                .height(24.0)
                .radius(Full)
                .fill(theme.surface)
                .cursor(Pointer)
                .align_child(Start, Center)
                .padding(Padding { top: 2.0, right: 3.0, bottom: 2.0, left: 3.0 })
                .child(
                    Rectangle::new()
                        .width(20.0)
                        .height(20.0)
                        .radius(Full)
                        .fill(theme.secondary_text),
                )
        };

        let switch_area = Rectangle::new()
            .width(44.0)
            .height(24.0)
            .cursor(Pointer)
            .on_click(move |_| {
                if is_enabled {
                    if total_active > 1 {
                        let _ = Command::new("niri").args(["msg", "output", &name_clone, "off"]).output();
                    }
                } else {
                    let _ = Command::new("niri").args(["msg", "output", &name_clone, "on"]).output();
                }
            })
            .child(switch_pill);

        let top_row = Row::new(children![left_part, switch_area])
            .width(width - 28.0)
            .justify(SpaceBetween)
            .align(Center);

        // Divider
        let divider = Rectangle::new()
            .width(width - 28.0)
            .height(1.0)
            .fill(theme.border);

        // Bottom row: Brightness Slider underneath
        let slider_row = monitor_brightness_slider(theme, width - 28.0);

        let card = Rectangle::new()
            .width(width)
            .height(112.0)
            .radius(14.0)
            .fill(theme.hover_surface)
            .padding(Padding {
                top: 12.0,
                right: 14.0,
                bottom: 12.0,
                left: 14.0,
            })
            .child(
                Column::new(children![top_row, divider, slider_row])
                    .gap(10.0),
            );

        items.push(Box::new(card));
    }

    // Monitor Layout App Card / Button
    let layout_icon = Rectangle::new()
        .width(36.0)
        .height(36.0)
        .radius(10.0)
        .fill(theme.accent)
        .align_child(Center, Center)
        .child(
            Text::new("󰍺")
                .size(18.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.on_accent),
        );

    let title_text = Text::new("Monitor Layout")
        .size(13.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let desc_text = Text::new("Visual display arrangement editor with magnetic snapping")
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::Light)
        .color(theme.secondary_text);

    let text_col = Column::new(children![title_text, desc_text]).gap(2.0);

    let left = Row::new(children![layout_icon, text_col])
        .gap(12.0)
        .align(Center);

    let open_pill = Rectangle::new()
        .width(108.0)
        .height(28.0)
        .radius(Full)
        .fill(theme.surface)
        .cursor(Pointer)
        .padding(Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 14.0,
        })
        .on_click(|_| {
            open_monitor_layout();
        })
        .align_child(Center, Center)
        .child(
            Row::new(children![
                Text::new("Open Layout")
                    .size(11.5)
                    .font(fonts::BODY)
                    .weight(Weight::Medium)
                    .color(theme.accent),
                Text::new(" 󰅂")
                    .size(12.0)
                    .font(fonts::NERD)
                    .color(theme.accent),
            ])
            .align(Center),
        );

    let layout_card = Rectangle::new()
        .width(width)
        .height(56.0)
        .radius(14.0)
        .fill(theme.hover_surface)
        .cursor(Pointer)
        .padding(Padding {
            top: 10.0,
            right: 14.0,
            bottom: 10.0,
            left: 14.0,
        })
        .on_click(|_| {
            open_monitor_layout();
        })
        .child(
            Row::new(children![left, open_pill])
                .width(width - 28.0)
                .justify(SpaceBetween)
                .align(Center),
        );

        items.push(Box::new(layout_card));

    Column::new(items).gap(10.0)
}

fn open_monitor_layout() {
    Overlay::write().control_center.hide();
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/rcrout"));
    let script = format!("{home}/.config/amane/plugins/monitor-layout/monitor_layout.py");
    let cmd = format!("gtk-launch io.github.cjgarcia1229.monitor-layout 2>/dev/null || python3 '{script}'");
    amane::spawn(&cmd);
}

fn monitor_brightness_slider(theme: &Theme, width: f32) -> Row {
    let percent = Brightness::read().percent();
    let value = (percent as f32 / 100.0).clamp(0.01, 1.0);

    let sun_icon = Text::new("󰃠")
        .size(15.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.accent);

    let track_w = (width - 120.0).max(140.0);
    let track_h = 6.0;
    let thumb_size = 18.0;

    let filled = Rectangle::new()
        .width(track_w * value)
        .height(track_h)
        .radius(Full)
        .fill(theme.accent);

    let line = Rectangle::new()
        .width(track_w)
        .height(track_h)
        .radius(Full)
        .fill(theme.surface)
        .translate(0.0, (28.0 - track_h) / 2.0)
        .child(filled);

    let thumb_x = (track_w - thumb_size) * value;

    let thumb = Rectangle::new()
        .width(thumb_size)
        .height(thumb_size)
        .radius(Full)
        .fill(theme.on_accent)
        .translate(thumb_x, (28.0 - thumb_size) / 2.0);

    let area = Rectangle::new()
        .width(track_w)
        .height(28.0)
        .cursor(Pointer)
        .on_drag(move |point| {
            let along = (point.x / track_w).clamp(0.01, 1.0);
            let new_percent = (along * 100.0).round() as u8;
            if new_percent != Brightness::read().percent() {
                Brightness::set(new_percent);
            }
        })
        ;

    let slider_stack = Stack::new(children![line, thumb, area])
        .width(track_w)
        .height(28.0);

    let val_label = Text::new(format!("{percent}%"))
        .size(12.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let val_box = Rectangle::new()
        .width(44.0)
        .height(28.0)
        .align_child(amane::End, Center)
        .child(val_label);

    Row::new(children![sun_icon, slider_stack, val_box])
        .width(width)
        .gap(12.0)
        .align(Center)
}

// ---------------- Network Card ----------------

fn network_tab(_overlay: &Overlay, theme: &Theme, width: f32) -> Column {
    let network = Network::read();
    let enabled = network.wifi_enabled();
    let current_ssid = network.ssid();

    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    // 1. Wi-Fi master switch card
    let icon_box = Rectangle::new()
        .width(36.0)
        .height(36.0)
        .radius(10.0)
        .fill(if enabled { theme.accent } else { theme.surface })
        .align_child(Center, Center)
        .child(
            Text::new("󰖩")
                .size(18.0)
                .font(fonts::NERD)
                .tight()
                .color(if enabled { theme.on_accent } else { theme.secondary_text }),
        );

    let (status_text, status_color) = if !enabled {
        (String::from("Wi-Fi is disabled"), theme.secondary_text)
    } else if !current_ssid.is_empty() {
        (format!("Connected to {current_ssid}"), theme.success)
    } else {
        (String::from("Not connected to any network"), theme.secondary_text)
    };

    let title_w = Text::new("Wi-Fi Wireless")
        .size(14.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let status_w = Text::new(status_text)
        .size(11.5)
        .font(fonts::BODY)
        .color(status_color);

    let text_col = Column::new(children![title_w, status_w]).gap(2.0);

    let left = Row::new(children![icon_box, text_col])
        .gap(12.0)
        .align(Center);

    let switch_pill = if enabled {
        Rectangle::new()
            .width(44.0)
            .height(24.0)
            .radius(Full)
            .fill(theme.accent)
            .cursor(Pointer)
            .align_child(amane::End, Center)
            .padding(Padding { top: 2.0, right: 3.0, bottom: 2.0, left: 3.0 })
            .child(Rectangle::new().width(20.0).height(20.0).radius(Full).fill(theme.on_accent))
    } else {
        Rectangle::new()
            .width(44.0)
            .height(24.0)
            .radius(Full)
            .fill(theme.surface)
            .cursor(Pointer)
            .align_child(Start, Center)
            .padding(Padding { top: 2.0, right: 3.0, bottom: 2.0, left: 3.0 })
            .child(Rectangle::new().width(20.0).height(20.0).radius(Full).fill(theme.secondary_text))
    };

    let switch_btn = Rectangle::new()
        .width(44.0)
        .height(24.0)
        .cursor(Pointer)
        .on_click(move |_| {
            Network::set_wifi(!enabled);
        })
        .child(switch_pill);

    let header_card = Rectangle::new()
        .width(width)
        .height(56.0)
        .radius(14.0)
        .fill(theme.hover_surface)
        .padding(Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 })
        .child(
            Row::new(children![left, switch_btn])
                .width(width - 28.0)
                .justify(SpaceBetween)
                .align(Center),
        );

    items.push(Box::new(header_card));

    if enabled {
        // Disconnect button if connected
        if !current_ssid.is_empty() {
            let disc_btn = Rectangle::new()
                .width(96.0)
                .height(28.0)
                .radius(Full)
                .fill(theme.surface)
                .cursor(Pointer)
                .on_click(|_| {
                    Network::disconnect();
                })
                .align_child(Center, Center)
                .child(
                    Text::new("Disconnect")
                        .size(11.5)
                        .font(fonts::BODY)
                        .weight(Weight::Medium)
                        .color(theme.danger),
                );

            let conn_card = Rectangle::new()
                .width(width)
                .height(48.0)
                .radius(12.0)
                .fill(theme.hover_surface)
                .padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 })
                .child(
                    Row::new(children![
                        Row::new(children![
                            Text::new("󰤨").size(16.0).font(fonts::NERD).color(theme.success),
                            Text::new(current_ssid).size(13.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text)
                        ]).gap(10.0).align(Center),
                        disc_btn
                    ])
                    .width(width - 28.0)
                    .justify(SpaceBetween)
                    .align(Center),
                );

            items.push(Box::new(conn_card));
        }

        // Available Networks list
        let aps = network.access_points();
        if !aps.is_empty() {
            let mut seen_ssids = std::collections::HashSet::new();
            let mut ap_cards: Vec<Box<dyn Widget>> = Vec::new();

            for ap in aps {
                let s = ap.ssid();
                if s.is_empty() || seen_ssids.contains(s) || s == current_ssid {
                    continue;
                }
                seen_ssids.insert(s.to_string());
                let s_clone = s.to_string();

                let signal_icon = match ap.strength() {
                    s if s > 75 => "󰤨",
                    s if s > 50 => "󰤥",
                    s if s > 25 => "󰤢",
                    _ => "󰤟",
                };

                let ap_left = Row::new(children![
                    Text::new(signal_icon).size(15.0).font(fonts::NERD).color(theme.secondary_text),
                    Text::new(s).size(12.5).font(fonts::BODY).weight(Weight::Medium).color(theme.text)
                ]).gap(10.0).align(Center);

                let connect_btn = Rectangle::new()
                    .width(76.0)
                    .height(26.0)
                    .radius(Full)
                    .fill(theme.surface)
                    .cursor(Pointer)
                    .on_click(move |_| {
                        Network::connect(&s_clone, None);
                    })
                    .align_child(Center, Center)
                    .child(
                        Text::new("Connect")
                            .size(11.0)
                            .font(fonts::BODY)
                            .weight(Weight::Medium)
                            .color(theme.accent),
                    );

                let ap_row = Rectangle::new()
                    .width(width)
                    .height(44.0)
                    .radius(10.0)
                    .fill(theme.hover_surface)
                    .padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 })
                    .child(
                        Row::new(children![ap_left, connect_btn])
                            .width(width - 24.0)
                            .justify(SpaceBetween)
                            .align(Center),
                    );

                ap_cards.push(Box::new(ap_row));
                if ap_cards.len() >= 6 {
                    break;
                }
            }

            if !ap_cards.is_empty() {
                let heading = Row::new(children![
                    Text::new("Available Networks")
                        .size(12.0)
                        .font(fonts::BODY)
                        .weight(Weight::SemiBold)
                        .color(theme.secondary_text),
                    Rectangle::new()
                        .width(24.0)
                        .height(24.0)
                        .radius(Full)
                        .cursor(Pointer)
                        .on_click(|_| {
                            Network::scan();
                        })
                        .align_child(Center, Center)
                        .child(
                            Text::new("󰑐")
                                .size(13.0)
                                .font(fonts::NERD)
                                .color(theme.accent),
                        )
                ])
                .width(width)
                .justify(SpaceBetween)
                .align(Center);

                items.push(Box::new(heading));
                for c in ap_cards {
                    items.push(c);
                }
            }
        }
    }

    Column::new(items).gap(8.0)
}

// ---------------- Bluetooth Card ----------------

fn bluetooth_tab(_overlay: &Overlay, theme: &Theme, width: f32) -> Column {
    let bluetooth = Bluetooth::read();
    let powered = bluetooth.powered();

    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    // 1. Bluetooth master switch card
    let icon_box = Rectangle::new()
        .width(36.0)
        .height(36.0)
        .radius(10.0)
        .fill(if powered { theme.accent } else { theme.surface })
        .align_child(Center, Center)
        .child(
            Text::new("󰂯")
                .size(18.0)
                .font(fonts::NERD)
                .tight()
                .color(if powered { theme.on_accent } else { theme.secondary_text }),
        );

    let (status_text, status_color) = if !bluetooth.available() {
        ("No Bluetooth adapter found", theme.secondary_text)
    } else if !powered {
        ("Bluetooth is disabled", theme.secondary_text)
    } else if bluetooth.scanning() {
        ("Scanning for devices…", theme.accent)
    } else {
        ("Ready for connections", theme.success)
    };

    let title_w = Text::new("Bluetooth")
        .size(14.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let status_w = Text::new(status_text)
        .size(11.5)
        .font(fonts::BODY)
        .color(status_color);

    let text_col = Column::new(children![title_w, status_w]).gap(2.0);

    let left = Row::new(children![icon_box, text_col])
        .gap(12.0)
        .align(Center);

    let switch_pill = if powered {
        Rectangle::new()
            .width(44.0)
            .height(24.0)
            .radius(Full)
            .fill(theme.accent)
            .cursor(Pointer)
            .align_child(amane::End, Center)
            .padding(Padding { top: 2.0, right: 3.0, bottom: 2.0, left: 3.0 })
            .child(Rectangle::new().width(20.0).height(20.0).radius(Full).fill(theme.on_accent))
    } else {
        Rectangle::new()
            .width(44.0)
            .height(24.0)
            .radius(Full)
            .fill(theme.surface)
            .cursor(Pointer)
            .align_child(Start, Center)
            .padding(Padding { top: 2.0, right: 3.0, bottom: 2.0, left: 3.0 })
            .child(Rectangle::new().width(20.0).height(20.0).radius(Full).fill(theme.secondary_text))
    };

    let switch_btn = Rectangle::new()
        .width(44.0)
        .height(24.0)
        .cursor(Pointer)
        .on_click(move |_| {
            Bluetooth::set_powered(!powered);
        })
        .child(switch_pill);

    let header_card = Rectangle::new()
        .width(width)
        .height(56.0)
        .radius(14.0)
        .fill(theme.hover_surface)
        .padding(Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 })
        .child(
            Row::new(children![left, switch_btn])
                .width(width - 28.0)
                .justify(SpaceBetween)
                .align(Center),
        );

    items.push(Box::new(header_card));

    if powered {
        let devices = bluetooth.devices();

        let heading = Row::new(children![
            Text::new("Paired & Nearby Devices")
                .size(12.0)
                .font(fonts::BODY)
                .weight(Weight::SemiBold)
                .color(theme.secondary_text),
            Rectangle::new()
                .width(68.0)
                .height(24.0)
                .radius(Full)
                .fill(theme.surface)
                .cursor(Pointer)
                .on_click(|_| {
                    if Bluetooth::read().scanning() {
                        Bluetooth::stop_scan();
                    } else {
                        Bluetooth::start_scan();
                    }
                })
                .align_child(Center, Center)
                .child(
                    Row::new(children![
                        Text::new("󰑐").size(11.0).font(fonts::NERD).color(theme.accent),
                        Text::new("Scan").size(11.0).font(fonts::BODY).weight(Weight::Medium).color(theme.text)
                    ]).gap(4.0).align(Center)
                )
        ])
        .width(width)
        .justify(SpaceBetween)
        .align(Center);

        items.push(Box::new(heading));

        if devices.is_empty() {
            let empty = Rectangle::new()
                .width(width)
                .height(60.0)
                .radius(12.0)
                .fill(theme.hover_surface)
                .align_child(Center, Center)
                .child(
                    Text::new("No Bluetooth devices found. Click Scan to search.")
                        .size(12.0)
                        .font(fonts::BODY)
                        .color(theme.secondary_text),
                );
            items.push(Box::new(empty));
        } else {
            for device in devices {
                let dev_name = device.name().to_string();
                let is_connected = device.connected();
                let dev_address = device.address().to_string();

                let dev_icon = match device.icon() {
                    "audio-headphones" | "audio-headset" => "󰋋",
                    "input-keyboard" => "󰌌",
                    "input-mouse" => "󰍽",
                    "phone" => "󰏲",
                    _ => "󰂯",
                };

                let dev_left = Row::new(children![
                    Text::new(dev_icon).size(16.0).font(fonts::NERD).color(if is_connected { theme.accent } else { theme.secondary_text }),
                    Column::new(children![
                        Text::new(&dev_name).size(12.5).font(fonts::BODY).weight(Weight::Medium).color(theme.text),
                        Text::new(if is_connected { "Connected" } else { "Paired" })
                            .size(10.0)
                            .font(fonts::BODY)
                            .color(if is_connected { theme.success } else { theme.secondary_text })
                    ]).gap(1.0)
                ]).gap(10.0).align(Center);

                let action_btn = Rectangle::new()
                    .width(84.0)
                    .height(26.0)
                    .radius(Full)
                    .fill(theme.surface)
                    .cursor(Pointer)
                    .on_click(move |_| {
                        if is_connected {
                            Bluetooth::disconnect(&dev_address);
                        } else {
                            Bluetooth::connect(&dev_address);
                        }
                    })
                    .align_child(Center, Center)
                    .child(
                        Text::new(if is_connected { "Disconnect" } else { "Connect" })
                            .size(11.0)
                            .font(fonts::BODY)
                            .weight(Weight::Medium)
                            .color(if is_connected { theme.danger } else { theme.accent }),
                    );

                let dev_row = Rectangle::new()
                    .width(width)
                    .height(48.0)
                    .radius(10.0)
                    .fill(theme.hover_surface)
                    .padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 })
                    .child(
                        Row::new(children![dev_left, action_btn])
                            .width(width - 24.0)
                            .justify(SpaceBetween)
                            .align(Center),
                    );

                items.push(Box::new(dev_row));
            }
        }
    }

    Column::new(items).gap(8.0)
}

// ---------------- Tabs Content (Appearance, Bar, Launcher, Input, Integrations) ----------------

fn appearance_tab(theme: &Theme, width: f32) -> Column {
    let settings = Settings::read();

    let color_mode = settings.text("color_mode");
    let mode_row = row_choice(
        theme,
        width,
        "Color Mode",
        "Interface light or dark style",
        vec![
            ("Auto", color_mode == "auto", "color_mode", "auto"),
            ("Light", color_mode == "light", "color_mode", "light"),
            ("Dark", color_mode == "dark", "color_mode", "dark"),
        ],
    );

    let speed = settings.text("animation_speed");
    let speed_row = row_choice(
        theme,
        width,
        "Animation Speed",
        "Motion and transition timing",
        vec![
            ("0.5×", speed == "0.5", "animation_speed", "0.5"),
            ("1.0×", speed == "1" || speed == "1.0", "animation_speed", "1"),
            ("1.5×", speed == "1.5", "animation_speed", "1.5"),
            ("2.0×", speed == "2" || speed == "2.0", "animation_speed", "2"),
        ],
    );

    let trans = settings.text("wallpaper_transition");
    let trans_row = row_choice(
        theme,
        width,
        "Wallpaper Transition",
        "Animation style when changing wallpaper",
        vec![
            ("Circle", trans == "circle", "wallpaper_transition", "circle"),
            ("Wipe", trans == "wipe", "wallpaper_transition", "wipe"),
            ("Wave", trans == "wave", "wallpaper_transition", "wave"),
            ("Fade", trans == "fade", "wallpaper_transition", "fade"),
        ],
    );

    let blur_row = row_switch(
        theme,
        width,
        "Reduce Transparency",
        "Opaque backgrounds for windows and panels",
        settings.flag("reduce_transparency"),
        "reduce_transparency",
    );

    let shake_row = row_switch(
        theme,
        width,
        "Shake to Find Cursor",
        "Enlarges cursor briefly when wiggling pointer",
        settings.flag("shake_cursor"),
        "shake_cursor",
    );

    Column::new(children![mode_row, speed_row, trans_row, blur_row, shake_row]).gap(8.0)
}

fn bar_tab(theme: &Theme, width: f32) -> Column {
    let settings = Settings::read();

    let logo_row = row_switch(
        theme,
        width,
        "Distro Logo Pill",
        "Show distribution icon on top left",
        settings.flag("bar_logo"),
        "bar_logo",
    );

    let tray_row = row_switch(
        theme,
        width,
        "System Tray Pill",
        "Show system tray icons on top right",
        settings.flag("bar_tray"),
        "bar_tray",
    );

    let clock_24_row = row_switch(
        theme,
        width,
        "24-Hour Clock",
        "Display status bar clock in 24-hour time",
        settings.flag("clock_24_hour"),
        "clock_24_hour",
    );

    let seconds_row = row_switch(
        theme,
        width,
        "Show Seconds",
        "Include seconds in the status bar clock",
        settings.flag("clock_seconds"),
        "clock_seconds",
    );

    let dismiss_row = row_switch(
        theme,
        width,
        "Click Outside to Dismiss",
        "Clicking empty space closes open pulldowns",
        settings.flag("click_outside_dismiss"),
        "click_outside_dismiss",
    );

    Column::new(children![logo_row, tray_row, clock_24_row, seconds_row, dismiss_row]).gap(8.0)
}

fn launcher_tab(theme: &Theme, width: f32) -> Column {
    let settings = Settings::read();

    let commands_row = row_switch(
        theme,
        width,
        "Command Mode",
        "Execute shell commands and actions in search",
        settings.flag("launcher_commands"),
        "launcher_commands",
    );

    let desc_row = row_switch(
        theme,
        width,
        "Descriptions",
        "Show application descriptions under titles",
        settings.flag("launcher_descriptions"),
        "launcher_descriptions",
    );

    let icons_row = row_switch(
        theme,
        width,
        "Application Icons",
        "Display graphical desktop icons beside search results",
        settings.flag("launcher_icons"),
        "launcher_icons",
    );

    let remember_row = row_switch(
        theme,
        width,
        "Remember Search Query",
        "Preserve previous search text when reopening launcher",
        settings.flag("launcher_remember_query"),
        "launcher_remember_query",
    );

    let close_row = row_switch(
        theme,
        width,
        "Close on Launch",
        "Dismiss launcher automatically after running an app",
        settings.flag("launcher_close_on_launch"),
        "launcher_close_on_launch",
    );

    Column::new(children![commands_row, desc_row, icons_row, remember_row, close_row]).gap(8.0)
}

fn input_tab(theme: &Theme, width: f32) -> Column {
    let settings = Settings::read();

    let tap_row = row_switch(
        theme,
        width,
        "Touchpad Tap to Click",
        "Tap surface to produce a primary click",
        settings.flag("input_touchpad_tap_to_click"),
        "input_touchpad_tap_to_click",
    );

    let nat_touch_row = row_switch(
        theme,
        width,
        "Touchpad Natural Scrolling",
        "Reverse scrolling direction for touchpads",
        settings.flag("input_touchpad_natural_scroll"),
        "input_touchpad_natural_scroll",
    );

    let dwt_row = row_switch(
        theme,
        width,
        "Disable While Typing",
        "Prevent accidental touchpad clicks while using keyboard",
        settings.flag("input_touchpad_dwt"),
        "input_touchpad_dwt",
    );

    let mouse_flat_row = row_switch(
        theme,
        width,
        "Mouse Flat Acceleration",
        "Use linear 1:1 mouse input without curve acceleration",
        settings.flag("input_mouse_flat"),
        "input_mouse_flat",
    );

    let left_handed_row = row_switch(
        theme,
        width,
        "Left-Handed Mouse",
        "Swap primary left and right click mouse buttons",
        settings.flag("input_mouse_left_handed"),
        "input_mouse_left_handed",
    );

    Column::new(children![tap_row, nat_touch_row, dwt_row, mouse_flat_row, left_handed_row]).gap(8.0)
}

fn integrations_tab(theme: &Theme, width: f32) -> Column {
    let settings = Settings::read();

    let cosmic_row = row_switch(
        theme,
        width,
        "COSMIC Theme Sync",
        "Sync shell color schemes to COSMIC desktop apps",
        settings.flag("integration_cosmic"),
        "integration_cosmic",
    );

    let zed_row = row_switch(
        theme,
        width,
        "Zed Editor Theme Sync",
        "Export matching color palette to Zed themes",
        settings.flag("integration_zed"),
        "integration_zed",
    );

    let strata_row = row_switch(
        theme,
        width,
        "Strata Theme Sync",
        "Export matching color palette to Strata file manager",
        settings.flag("integration_strata"),
        "integration_strata",
    );

    let gtk_row = row_switch(
        theme,
        width,
        "GTK Theme Integration",
        "Apply dynamic accent and styles to GTK 3 & 4 apps",
        settings.flag("integration_gtk"),
        "integration_gtk",
    );

    let term_row = row_switch(
        theme,
        width,
        "Terminal Color Sync",
        "Sync palette colors to Alacritty configuration",
        settings.flag("integration_terminal"),
        "integration_terminal",
    );

    let obsidian_row = row_switch(
        theme,
        width,
        "Obsidian Theme Sync",
        "Sync palette colors to Obsidian vault snippets",
        settings.flag("integration_obsidian"),
        "integration_obsidian",
    );

    Column::new(children![cosmic_row, zed_row, strata_row, gtk_row, term_row, obsidian_row]).gap(8.0)
}

// ---------------- Helpers: Switch & Choice Rows ----------------

fn row_switch(
    theme: &Theme,
    width: f32,
    title: &str,
    description: &str,
    checked: bool,
    key: &'static str,
) -> Rectangle {
    let title_text = Text::new(title)
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let desc_text = Text::new(description)
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::Light)
        .color(theme.secondary_text);

    let text_col = Column::new(children![title_text, desc_text]).gap(2.0);

    let switch_pill = if checked {
        Rectangle::new()
            .width(44.0)
            .height(24.0)
            .radius(Full)
            .fill(theme.accent)
            .cursor(Pointer)
            .align_child(amane::End, Center)
            .padding(Padding {
                top: 2.0,
                right: 3.0,
                bottom: 2.0,
                left: 3.0,
            })
            .child(
                Rectangle::new()
                    .width(20.0)
                    .height(20.0)
                    .radius(Full)
                    .fill(theme.on_accent),
            )
    } else {
        Rectangle::new()
            .width(44.0)
            .height(24.0)
            .radius(Full)
            .fill(theme.surface)
            .cursor(Pointer)
            .align_child(Start, Center)
            .padding(Padding {
                top: 2.0,
                right: 3.0,
                bottom: 2.0,
                left: 3.0,
            })
            .child(
                Rectangle::new()
                    .width(20.0)
                    .height(20.0)
                    .radius(Full)
                    .fill(theme.secondary_text),
            )
    };

    let card = Row::new(children![text_col, switch_pill])
        .width(width - 28.0)
        .justify(SpaceBetween)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(50.0)
        .radius(12.0)
        .fill(theme.hover_surface)
        .cursor(Pointer)
        .padding(Padding {
            top: 8.0,
            right: 14.0,
            bottom: 8.0,
            left: 14.0,
        })
        .on_click(move |_| {
            Settings::set(key, if checked { "false" } else { "true" });
        })
        .align_child(Center, Center)
        .child(card)
}

fn row_choice(
    theme: &Theme,
    width: f32,
    title: &str,
    description: &str,
    options: Vec<(&'static str, bool, &'static str, &'static str)>,
) -> Rectangle {
    let title_text = Text::new(title)
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let desc_text = Text::new(description)
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::Light)
        .color(theme.secondary_text);

    let text_col = Column::new(children![title_text, desc_text]).gap(2.0);

    let mut pills: Vec<Box<dyn Widget>> = Vec::new();
    for (label, selected, key, val) in options {
        let pill_w = pill::text_width(label, 11.5) + 20.0;
        let pill = Rectangle::new()
            .width(pill_w)
            .height(26.0)
            .radius(Full)
            .fill(if selected { theme.accent } else { theme.surface })
            .cursor(Pointer)
            .padding(Padding {
                top: 0.0,
                right: 10.0,
                bottom: 0.0,
                left: 10.0,
            })
            .on_click(move |_| {
                Settings::set(key, val);
            })
            .align_child(Center, Center)
            .child(
                Text::new(label)
                    .size(11.5)
                    .font(fonts::BODY)
                    .weight(if selected { Weight::SemiBold } else { Weight::Regular })
                    .color(if selected { theme.on_accent } else { theme.text }),
            );

        pills.push(Box::new(pill));
    }

    let pills_row = Row::new(pills).gap(6.0).align(Center);

    let card = Row::new(children![text_col, pills_row])
        .width(width - 28.0)
        .justify(SpaceBetween)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(52.0)
        .radius(12.0)
        .fill(theme.hover_surface)
        .padding(Padding {
            top: 8.0,
            right: 14.0,
            bottom: 8.0,
            left: 14.0,
        })
        .align_child(Center, Center)
        .child(card)
}

// IPC Command Handler for Settings Menu
pub fn ipc(arguments: &[String]) -> String {
    let command = arguments.first().map(String::as_str).unwrap_or("toggle");
    let monitor_arg = arguments.get(1).map(String::as_str);

    match command {
        "tab" => {
            if let Some(tab_name) = arguments.get(1).map(String::as_str) {
                let idx = match tab_name {
                    "monitors" | "0" => 0,
                    "network" | "wifi" | "1" => 1,
                    "bluetooth" | "bt" | "2" => 2,
                    "appearance" | "3" => 3,
                    "bar" | "4" => 4,
                    "launcher" | "5" => 5,
                    "input" | "6" => 6,
                    "integrations" | "7" => 7,
                    _ => 0,
                };
                Overlay::set_settings_tab(idx);
                Overlay::open_center_section(3, None);
            }
        }
        "show" => Overlay::open_center_section(3, monitor_arg),
        "hide" => Overlay::write().control_center.hide(),
        _ => Overlay::toggle_settings(monitor_arg),
    }

    String::from("ok")
}


// ---------------- Blue Light Filter (Night Light) ----------------

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct NightlightState {
    pub enabled: bool,
    pub temperature: u32,
}

impl Default for NightlightState {
    fn default() -> Self {
        Self {
            enabled: false,
            temperature: 4000,
        }
    }
}

pub fn nightlight_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/rcrout"));
    PathBuf::from(format!("{home}/.local/state/amane/nightlight.json"))
}

pub fn load_nightlight() -> NightlightState {
    let path = nightlight_path();
    if let Ok(bytes) = fs::read(&path) {
        if let Ok(state) = serde_json::from_slice::<NightlightState>(&bytes) {
            return state;
        }
    }
    let running = Command::new("pgrep").arg("-x").arg("wlsunset").output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    NightlightState {
        enabled: running,
        temperature: 4000,
    }
}

pub fn save_nightlight(state: &NightlightState) {
    let path = nightlight_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(&path, json);
    }
}

pub fn apply_nightlight(state: &NightlightState) {
    if state.enabled {
        let temp = state.temperature.clamp(2500, 6000);
        let cmd = format!("pkill -x wlsunset 2>/dev/null; wlsunset -T 6500 -t {temp} -S 12:00 -s 12:00 >/dev/null 2>&1 &");
        amane::spawn(&cmd);
    } else {
        let cmd = "pkill -x wlsunset 2>/dev/null || true";
        amane::spawn(cmd);
    }
}

pub fn init_nightlight() {
    let state = load_nightlight();
    if state.enabled {
        apply_nightlight(&state);
    }
}

fn bluelight_filter_card(theme: &Theme, width: f32) -> Rectangle {
    let state = load_nightlight();
    let is_enabled = state.enabled;
    let temp = state.temperature.clamp(2500, 6000);

    let icon_box = Rectangle::new()
        .width(36.0)
        .height(36.0)
        .radius(10.0)
        .fill(if is_enabled { theme.accent } else { theme.surface })
        .align_child(Center, Center)
        .child(
            Text::new("\u{f0594}")
                .size(18.0)
                .font(fonts::NERD)
                .tight()
                .color(if is_enabled { theme.on_accent } else { theme.secondary_text }),
        );

    let title_text = Text::new("Blue Light Filter")
        .size(13.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let sub_text = Text::new(if is_enabled {
        format!("Active  •  Warm temperature {temp} K")
    } else {
        String::from("Reduce blue light and eye strain for night viewing")
    })
    .size(11.0)
    .font(fonts::BODY)
    .weight(Weight::Light)
    .color(if is_enabled { theme.accent } else { theme.secondary_text });

    let text_col = Column::new(children![title_text, sub_text]).gap(2.0);

    let left_part = Row::new(children![icon_box, text_col])
        .gap(12.0)
        .align(Center);

    let switch_pill = if is_enabled {
        Rectangle::new()
            .width(44.0)
            .height(24.0)
            .radius(Full)
            .fill(theme.accent)
            .cursor(Pointer)
            .align_child(amane::End, Center)
            .padding(Padding { top: 2.0, right: 3.0, bottom: 2.0, left: 3.0 })
            .child(
                Rectangle::new()
                    .width(20.0)
                    .height(20.0)
                    .radius(Full)
                    .fill(theme.on_accent),
            )
    } else {
        Rectangle::new()
            .width(44.0)
            .height(24.0)
            .radius(Full)
            .fill(theme.surface)
            .cursor(Pointer)
            .align_child(Start, Center)
            .padding(Padding { top: 2.0, right: 3.0, bottom: 2.0, left: 3.0 })
            .child(
                Rectangle::new()
                    .width(20.0)
                    .height(20.0)
                    .radius(Full)
                    .fill(theme.secondary_text),
            )
    };

    let switch_area = Rectangle::new()
        .width(44.0)
        .height(24.0)
        .cursor(Pointer)
        .on_click(move |_| {
            let mut s = load_nightlight();
            s.enabled = !s.enabled;
            save_nightlight(&s);
            apply_nightlight(&s);
        })
        .child(switch_pill);

    let top_row = Row::new(children![left_part, switch_area])
        .width(width - 28.0)
        .justify(SpaceBetween)
        .align(Center);

    let divider = Rectangle::new()
        .width(width - 28.0)
        .height(1.0)
        .fill(theme.border);

    let slider_row = bluelight_slider(theme, &state, width - 28.0);

    Rectangle::new()
        .width(width)
        .height(112.0)
        .radius(14.0)
        .fill(theme.hover_surface)
        .padding(Padding {
            top: 12.0,
            right: 14.0,
            bottom: 12.0,
            left: 14.0,
        })
        .child(
            Column::new(children![top_row, divider, slider_row])
                .gap(10.0),
        )
}

fn bluelight_slider(theme: &Theme, state: &NightlightState, width: f32) -> Row {
    let temp = state.temperature.clamp(2500, 6000);
    let value = ((temp - 2500) as f32 / 3500.0).clamp(0.01, 1.0);

    let moon_icon = Text::new("\u{f1a4c}")
        .size(15.0)
        .font(fonts::NERD)
        .tight()
        .color(if state.enabled { theme.accent } else { theme.secondary_text });

    let track_w = (width - 130.0).max(140.0);
    let track_h = 6.0;
    let thumb_size = 18.0;

    let filled = Rectangle::new()
        .width(track_w * value)
        .height(track_h)
        .radius(Full)
        .fill(if state.enabled { theme.accent } else { theme.surface });

    let line = Rectangle::new()
        .width(track_w)
        .height(track_h)
        .radius(Full)
        .fill(theme.surface)
        .translate(0.0, (28.0 - track_h) / 2.0)
        .child(filled);

    let thumb_x = (track_w - thumb_size) * value;

    let thumb = Rectangle::new()
        .width(thumb_size)
        .height(thumb_size)
        .radius(Full)
        .fill(if state.enabled { theme.on_accent } else { theme.secondary_text })
        .translate(thumb_x, (28.0 - thumb_size) / 2.0);

    let area = Rectangle::new()
        .width(track_w)
        .height(28.0)
        .cursor(Pointer)
        .on_drag(move |point| {
            let along = (point.x / track_w).clamp(0.0, 1.0);
            let raw_temp = (2500.0 + along * 3500.0).round() as u32;
            let new_temp = ((raw_temp + 25) / 50) * 50;
            let mut s = load_nightlight();
            if s.temperature != new_temp {
                s.temperature = new_temp;
                save_nightlight(&s);
                if s.enabled {
                    apply_nightlight(&s);
                }
            }
        });

    let slider_stack = Stack::new(children![line, thumb, area])
        .width(track_w)
        .height(28.0);

    let val_label = Text::new(format!("{temp} K"))
        .size(12.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let val_box = Rectangle::new()
        .width(60.0)
        .height(28.0)
        .align_child(amane::End, Center)
        .child(val_label);

    Row::new(children![moon_icon, slider_stack, val_box])
        .width(width)
        .gap(12.0)
        .align(Center)
}
