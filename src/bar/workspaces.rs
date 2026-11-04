use std::fs;
use std::sync::OnceLock;

use amane::{
    Align, Button, Center, Color, Full, Media, Monitor, Parent, Pointer, Rectangle, Row, Service, Stack, Start, Text,
    Weight, Widget, Workspace, Workspaces, children,
};

use super::{motion, pill, star};
use crate::fonts;
use crate::overlay::Overlay;
use crate::settings::Settings;
use crate::theme::Theme;

const INACTIVE: &str = "\u{f444}";
const MEDIA_ICON: &str = "󰎈";

// longer song text is cut off
const MEDIA_TEXT_WIDTH: f32 = 180.0;

// every workspace is a 26px slot, 4px apart, inside a 32px pill
const SLOT: f32 = 26.0;
const SLOT_GAP: f32 = 4.0;
const STRIP_HEIGHT: f32 = 32.0;
const STRIP_PADDING: f32 = 4.0;

// with the row's 10px gap this keeps 20px from the screen edge
const EDGE: f32 = 10.0;

static DISTRO_LOGO: OnceLock<&'static str> = OnceLock::new();

// Detects the current distro from /etc/os-release and returns its Nerd Font icon
fn distro_logo() -> &'static str {
    DISTRO_LOGO.get_or_init(|| {
        for path in ["/etc/os-release", "/usr/lib/os-release"] {
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    if let Some(id) = line.strip_prefix("ID=") {
                        let id = id.trim().trim_matches('"').trim_matches('\'').to_lowercase();
                        return match id.as_str() {
                            "fedora" => "\u{f30a}",
                            "nixos" => "\u{f313}",
                            "arch" | "cachyos" | "endeavouros" => "\u{f303}",
                            "ubuntu" => "\u{f31b}",
                            "debian" => "\u{f306}",
                            "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" => "\u{f314}",
                            "gentoo" => "\u{f30d}",
                            "void" => "\u{f32e}",
                            "alpine" => "\u{f300}",
                            "manjaro" => "\u{f312}",
                            "linuxmint" => "\u{f30e}",
                            "pop" => "\u{f32a}",
                            _ => "\u{f31a}",
                        };
                    }
                }
            }
        }
        "\u{f30a}"
    })
}

// Left section of the bar: Distro logo (opens app menu) and Settings button (opens settings pulldown)
pub fn view(monitor: &Monitor, theme: &Theme, width: f32, align: Align) -> Row {
    let monitor_name = monitor.name.clone();
    let logo = Rectangle::new()
        .width(34.0)
        .height(STRIP_HEIGHT)
        .radius(Full)
        .fill(theme.surface)
        .cursor(Pointer)
        .on_click(move |_| Overlay::toggle_app_menu(Some(&monitor_name)))
        .align_child(Center, Center)
        .child(
            Text::new(distro_logo())
                .size(20.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.accent),
        );

    let settings = Settings::read();

    let mut items: Vec<Box<dyn Widget>> = vec![Box::new(Rectangle::new().width(EDGE).height(1.0))];

    if settings.flag("bar_logo") {
        items.push(Box::new(logo));
    }

    Row::new(items)
        .width(width)
        .height(Parent)
        .gap(8.0)
        .justify(Start)
        .align(align)
}

pub fn workspaces_strip(monitor: &Monitor, theme: &Theme, style: &str) -> Rectangle {
    let workspaces = Workspaces::read();

    let mut own: Vec<&Workspace> = Vec::new();

    // each bar only shows the workspaces of its own monitor
    for workspace in workspaces.list() {
        if workspace.output() == Some(monitor.name.as_str()) {
            own.push(workspace);
        }
    }

    own.sort_by_key(|workspace| workspace.index());

    strip(monitor, &own, theme, style)
}

// Only shown while media is actively playing
pub fn media(monitor: &Monitor, theme: &Theme) -> Option<Rectangle> {
    let media = Media::read();

    if !media.playing() || media.title().is_empty() {
        return None;
    }

    let text = if media.artist().is_empty() {
        String::from(media.title())
    } else {
        format!("{} - {}", media.artist(), media.title())
    };

    let text_width = pill::text_width(&text, 13.0).min(MEDIA_TEXT_WIDTH);

    let icon = Text::new(MEDIA_ICON)
        .size(14.0)
        .font(fonts::NERD)
        .color(theme.success);

    let label = Rectangle::new()
        .width(text_width)
        .height(Parent)
        .align_child(amane::Start, Center)
        .child(pill::label(&text, 13.0, theme.text).elide());

    // the icon is about 9px wide, then a 7px gap and 20px of padding
    let width = 9.0 + 7.0 + text_width + 20.0;

    let monitor_name = monitor.name.clone();
    Some(
        pill::view(width, theme.surface)
            .cursor(Pointer)
            .on_click(move |button| match button {
                Button::Left => Overlay::toggle_control_center(Some(&monitor_name)),
                _ => Media::play_pause(),
            })
            .align_child(Center, Center)
            .child(Row::new(children![icon, label]).gap(7.0).align(Center)),
    )
}

// the dots or numbers, with the sliding accent highlight drawn over them
fn strip(monitor: &Monitor, workspaces: &[&Workspace], theme: &Theme, style: &str) -> Rectangle {
    let mut slots: Vec<Box<dyn Widget>> = Vec::new();

    let mut active = 0;

    for (position, workspace) in workspaces.iter().enumerate() {
        if workspace.active() {
            active = position;
        }

        slots.push(Box::new(slot(workspace, theme, style)));
    }

    let count = workspaces.len() as f32;

    let gaps = (count - 1.0).max(0.0);

    let content = count * SLOT + gaps * SLOT_GAP;

    let (offset, rotation) = motion::highlight(&monitor.name, active, SLOT + SLOT_GAP);

    let highlight = Rectangle::new()
        .width(SLOT)
        .height(SLOT)
        .radius(Full)
        .fill(theme.accent)
        .translate(offset, 0.0);

    let active_index = workspaces.get(active).map_or(1, |workspace| workspace.index() as usize);

    // the pill style spins a star in the highlight, numbers repeat the active one
    let highlight: Box<dyn Widget> = match style {
        "numbers" => Box::new(
            highlight
                .align_child(Center, Center)
                .child(number(active_index, theme.on_accent)),
        ),
        "dots" => Box::new(highlight),
        _ => Box::new(highlight.child(star::view(SLOT, rotation, theme.on_accent))),
    };

    // the highlight slides over the dots, which stay where they are
    let layers = Stack::new(vec![Box::new(Row::new(slots).gap(SLOT_GAP).align(Center)), highlight]);

    Rectangle::new()
        .width(content + STRIP_PADDING * 2.0)
        .height(STRIP_HEIGHT)
        .radius(Full)
        .fill(theme.selected_surface)
        .align_child(Center, Center)
        .child(layers)
}

// a small dot or its number, the highlight covers the active one
fn slot(workspace: &Workspace, theme: &Theme, style: &str) -> Rectangle {
    let id = workspace.id();

    let color = if workspace.urgent() {
        theme.danger
    } else if workspace.active() {
        Color::TRANSPARENT
    } else {
        theme.muted_text
    };

    let mark = if style == "numbers" {
        number(workspace.index() as usize, color)
    } else {
        Text::new(INACTIVE).size(11.0).font(fonts::SYMBOLS).color(color)
    };

    Rectangle::new()
        .width(SLOT)
        .height(SLOT)
        .cursor(Pointer)
        .on_click(move |_| Workspaces::focus(id))
        .align_child(Center, Center)
        .child(mark)
}

fn number(index: usize, color: Color) -> Text {
    Text::new(index.to_string())
        .size(12.0)
        .font(fonts::BODY)
        .weight(Weight::Bold)
        .tight()
        .color(color)
}
