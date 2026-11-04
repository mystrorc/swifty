use std::cell::RefCell;

use amane::{
    Align, Battery, Button, Center, Color, End, Full, Link, Memory, Monitor, Network, Padding, Parent, Pointer, Rectangle, Row, Service,
    Stack, Text, Widget, children,
};

use super::{pill, ring};
use crate::fonts;
use crate::motion::{self, Glide};
use crate::overlay::Overlay;
use crate::recorder;
use crate::settings::Settings;
use crate::theme::Theme;

// the memory icon, and the notification, network and bluetooth tray icons
const MEMORY_ICON: &str = "󰍛";
const NOTIF_ICON: &str = "󰂚";
const BLUETOOTH_ICON: &str = "󰂯";

const RING_SIZE: f32 = 24.0;
const RING_THICKNESS: f32 = 3.0;

// 3 icons (18px each) + 2 gaps (10px each) + 24px padding = 98px
const TRAY_PADDING: f32 = 12.0;
const TRAY_ICON_WIDTH: f32 = 18.0;
const TRAY_GAP: f32 = 10.0;
const TRAY_WIDTH: f32 = TRAY_PADDING * 2.0 + TRAY_ICON_WIDTH * 3.0 + TRAY_GAP * 2.0;

// with the row's 10px gap this keeps 15px from the screen edge
const EDGE: f32 = 5.0;

const RECORDING_DOT: &str = "\u{f0765}";

// milliseconds the tray takes to grow and shrink around the recording time
const RECORDING_DURATION: u64 = 420;

thread_local! {
    // kept outside services, a service write from view() would draw frames forever
    static RECORDING: RefCell<Option<(Glide, String)>> = const { RefCell::new(None) };
}

// Right section of the bar: System Tray and Power Menu button
pub fn view(monitor: &Monitor, theme: &Theme, width: f32, align: Align) -> Row {
    let settings = Settings::read();

    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    if settings.flag("bar_tray") {
        items.push(Box::new(tray(monitor, theme)));
    }

    // Power button removed from bar; integrated into Power tab in center dropdown

    items.push(Box::new(Rectangle::new().width(EDGE).height(1.0)));

    Row::new(items)
        .width(width)
        .height(Parent)
        .gap(10.0)
        .justify(End)
        .align(align)
}

#[allow(dead_code)]
fn power_button(monitor: &Monitor, theme: &Theme) -> Rectangle {
    let icon = Text::new("\u{f011}") // Nerd Font power icon
        .size(15.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.danger);

    let monitor_name = monitor.name.clone();
    Rectangle::new()
        .width(32.0)
        .height(32.0)
        .radius(Full)
        .fill(theme.surface)
        .cursor(Pointer)
        .on_click(move |_| Overlay::toggle_power_menu(Some(&monitor_name)))
        .align_child(Center, Center)
        .child(icon)
}

// a ring with a small icon inside, then the value
fn indicator(value: f32, color: Color, icon: &str, text: &str, theme: &Theme) -> Row {
    let ring = ring::view(RING_SIZE, RING_THICKNESS, value, color, theme.border);

    let icon = Rectangle::new()
        .width(RING_SIZE)
        .height(RING_SIZE)
        .align_child(Center, Center)
        .child(Text::new(icon).size(10.0).font(fonts::MATERIAL).color(theme.text));

    let ring_with_icon = Stack::new(children![ring, icon]);

    Row::new(children![ring_with_icon, pill::label(text, 14.0, theme.text)])
        .gap(6.0)
        .align(Center)
}

// Only shown when a battery is present and NOT on the charger (neither charging nor full/plugged-in)
pub fn battery(monitor: &Monitor, theme: &Theme) -> Option<Rectangle> {
    let battery = Battery::read();

    if !battery.present() || battery.charging() || battery.full() {
        return None;
    }

    let percent = battery.percent();

    let color = if percent > 20 {
        theme.accent
    } else {
        theme.danger
    };

    let icon = battery_icon(percent, false);

    let text = format!("{percent}%");

    let value = f32::from(percent) / 100.0;

    let ind = indicator(value, color, icon, &text, theme);

    let text_w = pill::text_width(&text, 14.0);
    let width = RING_SIZE + 6.0 + text_w + 16.0;

    let monitor_name = monitor.name.clone();
    Some(
        pill::view(width, theme.surface)
            .cursor(Pointer)
            .on_click(move |button| match button {
                Button::Left => Overlay::toggle_power_devices(Some(&monitor_name)),
                _ => {}
            })
            .align_child(Center, Center)
            .child(ind),
    )
}

// one glyph per 10%
fn battery_icon(percent: u8, charging: bool) -> &'static str {
    if charging {
        return "󱚥";
    }

    match percent {
        91.. => "󰁹",
        81..=90 => "󰂂",
        71..=80 => "󰂁",
        61..=70 => "󰂀",
        51..=60 => "󰁿",
        41..=50 => "󰁾",
        31..=40 => "󰁽",
        21..=30 => "󰁼",
        11..=20 => "󰁻",
        _ => "󰁺",
    }
}

// Segment inside the locked center pill for System (Memory)
pub fn memory_segment(monitor: &Monitor, theme: &Theme) -> (f32, Rectangle) {
    let memory = Memory::read();

    let usage = f32::from(memory.percent()) / 100.0;

    let color = if usage < 0.5 {
        theme.success
    } else if usage < 0.8 {
        theme.accent
    } else {
        theme.danger
    };

    // the used amount, like "5.2G"
    let gibibytes = memory.used_kib() as f32 / 1024.0 / 1024.0;

    let text = format!("{gibibytes:.1}G");

    let ind = indicator(usage, color, MEMORY_ICON, &text, theme);

    let text_w = pill::text_width(&text, 14.0);
    let width = RING_SIZE + 6.0 + text_w + 16.0;

    let monitor_name = monitor.name.clone();
    let seg = Rectangle::new()
        .width(width)
        .height(Parent)
        .cursor(Pointer)
        .on_click(move |button| match button {
            Button::Left => Overlay::toggle_sysmon(Some(&monitor_name)),
            _ => {}
        })
        .align_child(Center, Center)
        .child(ind);

    (width, seg)
}

fn network_icon() -> &'static str {
    let network = Network::read();
    if !network.connected() {
        "󰤭" // Disconnected icon
    } else {
        match network.link() {
            Link::Wired => "󰈀", // Ethernet LAN icon
            Link::Wifi => "",  // Wi-Fi icon
            _ => "󰈀",
        }
    }
}

fn tray_icon(icon: &str, theme: &Theme) -> Rectangle {
    Rectangle::new()
        .width(TRAY_ICON_WIDTH)
        .height(Parent)
        .align_child(Center, Center)
        .child(
            Text::new(icon)
                .size(15.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.on_accent),
        )
}

// opens the utility center, and grows to the left with the time while a recording runs
fn tray(monitor: &Monitor, theme: &Theme) -> Rectangle {
    let icons: Vec<Box<dyn Widget>> = vec![
        Box::new(tray_icon(NOTIF_ICON, theme)),
        Box::new(tray_icon(network_icon(), theme)),
        Box::new(tray_icon(BLUETOOTH_ICON, theme)),
    ];

    let row = Row::new(icons)
        .gap(TRAY_GAP)
        .align(Center);

    let (extra, recording) = recording(theme);

    let content = Row::new(children![recording, row])
        .gap(8.0)
        .align(Center);

    let monitor_name = monitor.name.clone();
    pill::view(TRAY_WIDTH + extra, theme.accent)
        .clip()
        .cursor(Pointer)
        .on_click(move |_| Overlay::toggle_utility(Some(&monitor_name)))
        .align_child(End, Center)
        .padding(Padding {
            top: 0.0,
            right: TRAY_PADDING,
            bottom: 0.0,
            left: TRAY_PADDING,
        })
        .child(content)
}

/*
 * the dot and running time in front of the tray icons, and how much wider it makes the pill;
 * the width glides, and the last time stays while it closes
 */
fn recording(theme: &Theme) -> (f32, Rectangle) {
    let status = recorder::status();

    RECORDING.with_borrow_mut(|shown| {
        let (width, text) =
            shown.get_or_insert_with(|| (motion::spatial(0.0, RECORDING_DURATION), String::new()));

        if let Some(time) = status.as_ref() {
            text.clone_from(time);
        }

        // 10px of padding, the dot, a 6px gap, the time, then 8px before the icons
        let full = 10.0 + 8.0 + 6.0 + pill::text_width(text, 13.0) + 8.0;

        width.to(if status.is_some() { full } else { 0.0 });

        let extra = width.value();

        let dot = Text::new(RECORDING_DOT).size(8.0).font(fonts::NERD).color(theme.on_accent);

        let segment = Rectangle::new()
            .width(extra)
            .height(Parent)
            .opacity(if full > 0.0 { extra / full } else { 0.0 })
            .align_child(Center, Center)
            .child(
                Row::new(children![dot, pill::label(text, 13.0, theme.on_accent)])
                    .gap(6.0)
                    .align(Center),
            );

        (extra, segment)
    })
}
