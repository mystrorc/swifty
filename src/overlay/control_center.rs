mod art;
pub mod audio_eq;
pub mod calendar;
mod cava;
mod fader;
mod media;
mod player;
mod visualizer;
pub mod wave;

use amane::{
    Audio, Center, Column, Padding, Pointer, Rectangle, Row, ScrollArea, Service, Start, Text,
    Weight, Widget, children,
};

use crate::fonts;
use crate::overlay::audio_control::AudioDevices;

use super::{Overlay, PanelView, Region, power_devices, settings_menu, sysmon};
use crate::liquid::Blob;
use crate::theme::{self, Theme};

const MAX_WIDTH: f32 = 820.0;

const TOP_BAR_HEIGHT: f32 = 36.0;
const TOP_BAR_GAP: f32 = 12.0;

// Scaled to fit modern layout with 7-day weather forecast, agenda reminders, and full audio controls
const CARD_HEIGHT: f32 = 470.0;

const PADDING: Padding = Padding {
    top: 18.0,
    right: 18.0,
    bottom: 18.0,
    left: 18.0,
};

// Self-contained floating card with all four corners rounded inward
const INNER_HEIGHT: f32 = TOP_BAR_HEIGHT + TOP_BAR_GAP + CARD_HEIGHT;
const HEIGHT: f32 = INNER_HEIGHT + PADDING.top + PADDING.bottom; // 554.0
const RADIUS: f32 = 28.0;

// Distance below the bar when open, and extra room for the drop shadow
const TOP_OFFSET: f32 = 8.0;
const SHADOW_MARGIN: f32 = 20.0;
const HIDDEN_MARGIN: f32 = 32.0;

const GAP: f32 = 14.0;
const CARD_RADIUS: f32 = 20.0;

const VOLUME_ICON: &str = "󱕾";
const MICROPHONE_ICON: &str = "󰍬";

pub fn view(overlay: &Overlay, theme: &Theme, screen: Region) -> Option<PanelView> {
    let progress = overlay.control_center.progress.value();

    if !overlay.control_center.shown && progress <= 0.001 {
        return None;
    }

    let width = MAX_WIDTH.min(screen.width - 64.0);

    let x = (screen.width - width) / 2.0;

    let hidden = -(HEIGHT + HIDDEN_MARGIN);

    let y = TOP_OFFSET + (hidden - TOP_OFFSET) * (1.0 - progress);

    let blob = Blob::new(x, y, width, HEIGHT)
        .radius(RADIUS)
        .child(content(overlay, theme, width));

    let input = Region {
        x,
        y,
        width,
        height: HEIGHT,
    };

    let reach = Region {
        x: x - SHADOW_MARGIN,
        y: 0.0,
        width: width + SHADOW_MARGIN * 2.0,
        height: TOP_OFFSET + HEIGHT + SHADOW_MARGIN,
    };

    Some(PanelView { blob, input, reach })
}

// Higher-level tabs at the top of the middle pulldown: Overview / System Monitor / Media / Power
fn section_tabs(overlay: &Overlay, theme: &Theme, width: f32) -> Row {
    let overview_btn = section_tab_button(
        theme,
        "\u{f073}",
        "Overview",
        overlay.center_tab == 0,
        || Overlay::set_center_tab(0),
    );
    let sysmon_btn = section_tab_button(
        theme,
        "󰍛",
        "System Monitor",
        overlay.center_tab == 1,
        || Overlay::set_center_tab(1),
    );
    let media_btn = section_tab_button(
        theme,
        "󰎆",
        "Media",
        overlay.center_tab == 2,
        || Overlay::set_center_tab(2),
    );
    let settings_btn = section_tab_button(
        theme,
        "\u{f0493}",
        "Settings",
        overlay.center_tab == 3,
        || Overlay::set_center_tab(3),
    );
    let power_btn = section_tab_button(
        theme,
        "\u{f011}",
        "Power",
        overlay.center_tab == 4,
        || Overlay::set_center_tab(4),
    );

    let strip = Rectangle::new()
        .width(652.0)
        .height(TOP_BAR_HEIGHT)
        .radius(14.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 4.0,
            right: 4.0,
            bottom: 4.0,
            left: 4.0,
        })
        .align_child(Center, Center)
        .child(
            Row::new(children![overview_btn, sysmon_btn, media_btn, settings_btn, power_btn])
                .gap(6.0)
                .align(Center),
        );

    Row::new(children![strip])
        .width(width)
        .height(TOP_BAR_HEIGHT)
        .justify(Center)
        .align(Center)
}

fn section_tab_button(
    theme: &Theme,
    icon: &str,
    label: &str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> Rectangle {
    let fill = if active {
        theme.accent
    } else {
        amane::Color::TRANSPARENT
    };
    let text_color = if active {
        theme.on_accent
    } else {
        theme.secondary_text
    };
    let icon_color = if active {
        theme.on_accent
    } else {
        theme.accent
    };

    let icon_w = Text::new(icon)
        .size(14.0)
        .font(fonts::NERD)
        .tight()
        .color(icon_color);

    let label_w = Text::new(label)
        .size(12.0)
        .font(fonts::BODY)
        .weight(if active { Weight::SemiBold } else { Weight::Medium })
        .color(text_color);

    let row = Row::new(children![icon_w, label_w]).gap(6.0).align(Center);

    Rectangle::new()
        .width(124.0)
        .height(28.0)
        .radius(10.0)
        .fill(fill)
        .cursor(Pointer)
        .on_click(move |_| on_click())
        .align_child(Center, Center)
        .child(row)
}

#[allow(dead_code)]
pub fn tabs(overlay: &Overlay, theme: &Theme) -> Row {
    let now_playing = tab_button(
        theme,
        "󰎆",
        "Now Playing",
        overlay.control_tab == 0,
        || Overlay::set_control_tab(0),
    );
    let audio_eq = tab_button(
        theme,
        "󱓃",
        "Audio & EQ",
        overlay.control_tab == 1,
        || Overlay::set_control_tab(1),
    );

    Row::new(children![now_playing, audio_eq])
        .gap(6.0)
        .align(Center)
}

#[allow(dead_code)]
fn tab_button(
    theme: &Theme,
    icon: &str,
    label: &str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> Rectangle {
    let fill = if active {
        theme.selected_surface
    } else {
        amane::Color::TRANSPARENT
    };
    let text_color = if active {
        theme.text
    } else {
        theme.muted_text
    };

    let icon_w = Text::new(icon)
        .size(13.0)
        .font(fonts::NERD)
        .tight()
        .color(if active { theme.accent } else { theme.secondary_text });

    let label_w = Text::new(label)
        .size(12.0)
        .font(fonts::BODY)
        .weight(if active { Weight::SemiBold } else { Weight::Regular })
        .color(text_color);

    let row = Row::new(children![icon_w, label_w]).gap(6.0).align(Center);
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
        .on_click(move |_| on_click())
        .align_child(Center, Center)
        .child(row)
}

fn content(overlay: &Overlay, theme: &Theme, width: f32) -> Rectangle {
    let inner_width = width - PADDING.left - PADDING.right;

    let top_tabs = section_tabs(overlay, theme, inner_width);

    let body: Box<dyn Widget> = match overlay.center_tab {
        0 => Box::new(calendar::embedded_view(overlay, theme, inner_width, CARD_HEIGHT)),
        1 => Box::new(sysmon::embedded_view(theme, inner_width, CARD_HEIGHT)),
        3 => Box::new(settings_menu::embedded_view(overlay, theme, inner_width, CARD_HEIGHT)),
        4 => Box::new(power_devices::embedded_view(theme, inner_width, CARD_HEIGHT)),
        _ => {
            let left_width: f32 = 220.0;
            let card_width = inner_width - left_width - GAP;

            let left_card = sound_control_card(theme, left_width, CARD_HEIGHT);
            let right_card = media::view(overlay, theme, card_width, CARD_HEIGHT);

            Box::new(Row::new(vec![Box::new(left_card) as Box<dyn Widget>, Box::new(right_card) as Box<dyn Widget>]).gap(GAP))
        }
    };

    let column = Column::new(vec![Box::new(top_tabs) as Box<dyn Widget>, body])
        .width(inner_width)
        .gap(TOP_BAR_GAP);

    Rectangle::new()
        .width(width)
        .height(HEIGHT)
        .padding(PADDING)
        .on_hover(Overlay::hover_control_center)
        .child(column)
}

// Left card: Volume & Mic sliders on top, Output selector underneath
fn sound_control_card(theme: &Theme, width: f32, height: f32) -> Rectangle {
    let faders_h = 165.0;
    let gap = 10.0;
    let divider_h = 1.0;
    let outputs_h = (height - faders_h - divider_h - gap * 2.0 - 24.0).max(100.0);

    let faders_part = sound_faders(theme, faders_h);
    let sep = Rectangle::new()
        .width(width - 24.0)
        .height(divider_h)
        .fill(theme::with_opacity(theme.border, 0.5));
    let outputs_part = outputs_view(theme, width - 20.0, outputs_h);

    let col = Column::new(children![faders_part, sep, outputs_part])
        .gap(gap)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: 12.0,
            bottom: 12.0,
            left: 10.0,
            right: 10.0,
        })
        .align_child(Center, Start)
        .child(col)
}

fn sound_faders(theme: &Theme, height: f32) -> Row {
    let audio = Audio::read();

    let volume = fader::Fader {
        icon: VOLUME_ICON,
        level: audio.volume(),
        muted: audio.muted(),
        set: Audio::set_volume,
        toggle_mute: Audio::toggle_mute,
    };

    let microphone = fader::Fader {
        icon: MICROPHONE_ICON,
        level: audio.microphone_volume(),
        muted: audio.microphone_muted(),
        set: Audio::set_microphone_volume,
        toggle_mute: Audio::toggle_microphone_mute,
    };

    Row::new(children![
        fader::view(volume, theme, height),
        fader::view(microphone, theme, height),
    ])
    .gap(16.0)
    .align(Center)
}

fn outputs_view(theme: &Theme, width: f32, height: f32) -> Column {
    let devices = AudioDevices::read();

    let header_icon = Text::new("󰓃").size(12.0).font(fonts::NERD).tight().color(theme.accent);
    let count_text = format!("Outputs ({})", devices.sinks.len());
    let header_text = Text::new(&count_text).size(12.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text);
    let header = Row::new(children![header_icon, header_text]).gap(6.0).align(Center);

    let mut sink_items: Vec<Box<dyn Widget>> = Vec::new();
    if devices.sinks.is_empty() {
        let empty = Text::new("No outputs detected")
            .size(11.0)
            .font(fonts::BODY)
            .color(theme.muted_text);
        sink_items.push(Box::new(empty));
    } else {
        for sink in &devices.sinks {
            sink_items.push(Box::new(audio_eq::sink_item(theme, sink, width)));
        }
    }

    let scroll_h = (height - 18.0 - 24.0 - 12.0).max(60.0);
    let sinks_column = Column::new(sink_items).width(width).gap(4.0);
    let sinks_scroll = ScrollArea::new("control-center-outputs-scroll", sinks_column)
        .width(width)
        .height(scroll_h);

    let mixer_icon = Text::new("󰓃").size(11.0).font(fonts::NERD).tight().color(theme.secondary_text);
    let mixer_label = Text::new("Open Mixer").size(11.0).font(fonts::BODY).color(theme.secondary_text);
    let mixer_btn = Rectangle::new()
        .width(width)
        .height(24.0)
        .radius(6.0)
        .fill(theme.selected_surface)
        .cursor(Pointer)
        .on_click(|_| {
            let _ = std::process::Command::new("ghostty").args(["-e", "wiremix"]).spawn();
        })
        .align_child(Center, Center)
        .child(Row::new(children![mixer_icon, mixer_label]).gap(4.0).align(Center));

    Column::new(children![header, sinks_scroll, mixer_btn])
        .width(width)
        .gap(6.0)
}

// "toggle", "show" or "hide"
pub fn ipc(arguments: &[String]) -> String {
    let command = arguments.first().map(String::as_str).unwrap_or("toggle");
    let monitor_arg = arguments.get(1).map(String::as_str);

    let shown = Overlay::read().control_center.shown && Overlay::read().center_tab == 2;

    let wanted = match command {
        "show" => true,
        "hide" => false,
        _ => !shown,
    };

    if wanted {
        Overlay::open_control_tab(0, monitor_arg);
    } else {
        Overlay::write().control_center.hide();
    }

    String::from("ok")
}
