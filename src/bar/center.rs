use amane::{
    Align, Audio, Button, Center, End, Monitor, Parent, Pointer, Rectangle, Row, Scroll, Service, Stack,
    Start, Text, Weight, Widget, children,
};

use super::{pill, ring, system, timer, workspaces};
use crate::clock::Clock;
use crate::fonts;
use crate::overlay::Overlay;
use crate::settings::Settings;
use crate::theme::Theme;

const RING_SIZE: f32 = 24.0;
const RING_THICKNESS: f32 = 3.0;

// how far one wheel step moves a level
const STEP: i32 = 5;

pub fn view(monitor: &Monitor, theme: &Theme, width: f32, align: Align) -> Row {
    let settings = Settings::read();

    let show_workspaces = settings.flag("bar_workspaces");
    let workspace_style = settings.text("workspace_style").to_string();
    let show_media = settings.flag("bar_media");
    let show_audio = settings.flag("bar_audio");
    let show_clock = settings.flag("bar_clock");
    let show_battery = settings.flag("bar_battery");
    let show_memory = settings.flag("bar_memory");

    // the clock reads the settings again, so this read ends first
    drop(settings);

    // Build the combined center pill (Volume + Time + System) locked in the exact center of the screen
    let mut core_segments: Vec<(f32, Box<dyn Widget>)> = Vec::new();

    if show_audio {
        let (w, seg) = audio_segment(monitor, theme);
        core_segments.push((w, Box::new(seg)));
    }

    if show_clock {
        let (w, seg) = clock_segment(monitor, theme);
        core_segments.push((w, Box::new(seg)));
    }

    if show_memory {
        let (w, seg) = system::memory_segment(monitor, theme);
        core_segments.push((w, Box::new(seg)));
    }

    let mut core_width = 0.0;
    let mut core_items: Vec<Box<dyn Widget>> = Vec::new();

    if !core_segments.is_empty() {
        core_width += 8.0; // 4px outer padding on each end of the combined pill
        for (idx, (seg_w, seg)) in core_segments.into_iter().enumerate() {
            if idx > 0 {
                core_items.push(Box::new(
                    Rectangle::new()
                        .width(1.0)
                        .height(14.0)
                        .fill(theme.border),
                ));
                core_width += 1.0;
            }
            core_items.push(seg);
            core_width += seg_w;
        }
    }

    // Left wing (Workspaces, Timer, Media) aligned to the right edge of the left wing (immediately left of the center pill)
    let mut left_items: Vec<Box<dyn Widget>> = Vec::new();

    if show_workspaces {
        left_items.push(Box::new(workspaces::workspaces_strip(
            monitor,
            theme,
            &workspace_style,
        )));
    }

    if let Some(timer) = timer::view(theme) {
        left_items.push(Box::new(timer));
    }

    if show_media {
        if let Some(media) = workspaces::media(monitor, theme) {
            left_items.push(Box::new(media));
        }
    }

    // Right wing (Battery) aligned to the left edge of the right wing (immediately right of the center pill)
    let mut right_items: Vec<Box<dyn Widget>> = Vec::new();

    if show_battery {
        if let Some(battery) = system::battery(monitor, theme) {
            right_items.push(Box::new(battery));
        }
    }

    if let Some(ha_pill) = crate::bar::home_assistant::pill(monitor, theme) {
        right_items.push(Box::new(ha_pill));
    }

    let gap = 8.0;
    let wing_width = ((width - core_width - gap * 2.0) / 2.0).max(0.0);

    let left_wing = Row::new(left_items)
        .width(wing_width)
        .height(Parent)
        .gap(8.0)
        .justify(End)
        .align(align);

    let right_wing = Row::new(right_items)
        .width(wing_width)
        .height(Parent)
        .gap(8.0)
        .justify(Start)
        .align(align);

    let center_locked: Box<dyn Widget> = if core_width > 0.0 {
        Box::new(
            pill::view(core_width, theme.surface)
                .align_child(Center, Center)
                .child(Row::new(core_items).height(Parent).align(Center)),
        )
    } else {
        Box::new(Rectangle::new().width(0.0).height(1.0))
    };

    Row::new(vec![
        Box::new(left_wing),
        center_locked,
        Box::new(right_wing),
    ])
    .width(width)
    .height(Parent)
    .gap(gap)
    .justify(Center)
    .align(align)
}

// Volume segment inside the locked center pill
fn audio_segment(monitor: &Monitor, theme: &Theme) -> (f32, Rectangle) {
    let audio = Audio::read();
    let volume = audio.volume();
    let muted = audio.muted();

    let speaker_color = if muted {
        theme.muted_text
    } else {
        theme.accent
    };

    let text_color = if muted {
        theme.muted_text
    } else {
        theme.text
    };

    let icon_glyph = if muted || volume == 0 {
        "\u{f075f}"
    } else if volume < 35 {
        "\u{f057f}"
    } else if volume < 70 {
        "\u{f0580}"
    } else {
        "\u{f057e}"
    };

    let speaker_value = f32::from(volume) / 100.0;

    let ring = ring::view(RING_SIZE, RING_THICKNESS, speaker_value, speaker_color, theme.border);

    let icon = Rectangle::new()
        .width(RING_SIZE)
        .height(RING_SIZE)
        .align_child(Center, Center)
        .child(Text::new(icon_glyph).size(11.0).font(fonts::MATERIAL).color(text_color));

    let ring_with_icon = Stack::new(children![ring, icon]);

    let text = format!("{volume}%");
    let text_w = pill::text_width(&text, 14.0);

    let ind = Row::new(children![ring_with_icon, pill::label(&text, 14.0, text_color)])
        .gap(6.0)
        .align(Center);

    let width = RING_SIZE + 6.0 + text_w + 16.0;

    let monitor_name = monitor.name.clone();
    let seg = Rectangle::new()
        .width(width)
        .height(Parent)
        .cursor(Pointer)
        .on_scroll(scroll_volume)
        .on_click(move |button| match button {
            Button::Left => Overlay::toggle_control_center(Some(&monitor_name)),
            Button::Right => {
                let is_eq_open = {
                    let o = Overlay::read();
                    o.control_center.shown && o.center_tab == 2 && o.control_tab == 1
                };
                if is_eq_open {
                    Overlay::toggle_control_center(Some(&monitor_name));
                } else {
                    Overlay::open_control_tab(1, Some(&monitor_name));
                }
            }
            _ => {}
        })
        .align_child(Center, Center)
        .child(ind);

    (width, seg)
}

// Clock segment inside the locked center pill: Left click pulls down center pulldown on Calendar tab
fn clock_segment(monitor: &Monitor, theme: &Theme) -> (f32, Rectangle) {
    let clock = Clock::read();
    let settings = Settings::read();

    let seconds = settings.flag("clock_seconds");
    let display = settings.text("clock_display");

    let text = match display {
        "time" => clock.time(seconds),
        "date" => String::from(clock.date()),
        _ => format!("{}  \u{2022}  {}", clock.time(seconds), clock.date()),
    };

    drop(settings);

    let text_width = pill::text_width(&text, 14.0);
    let width = text_width + 24.0;

    let monitor_name = monitor.name.clone();
    let seg = Rectangle::new()
        .width(width)
        .height(Parent)
        .cursor(Pointer)
        .on_click(move |button| match button {
            Button::Left => Overlay::toggle_calendar(Some(&monitor_name)),
            _ => {}
        })
        .align_child(Center, Center)
        .child(
            Text::new(&text)
                .size(14.0)
                .font(fonts::BODY)
                .weight(Weight::Bold)
                .color(theme.text),
        );

    (width, seg)
}

fn scroll_volume(scroll: Scroll) {
    let volume = i32::from(Audio::read().volume());

    Audio::set_volume(step(volume, scroll));
}

// wheel up is a negative y, which turns the level up
fn step(level: i32, scroll: Scroll) -> u8 {
    let change = if scroll.y < 0.0 { STEP } else { -STEP };

    (level + change).clamp(0, 100) as u8
}
