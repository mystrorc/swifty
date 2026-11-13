use amane::{
    Center, Color, Column, Padding, Pointer, Rectangle, Row, ScrollArea, Service,
    SpaceBetween, Start, Text, Weight, Widget, children,
};

use super::header;
use super::{hover, hovered};
use crate::fonts;
use crate::overlay::Overlay;
use crate::settings::Settings;
use crate::settings::pages::appearance::{CURSORS, FONTS, SIZES};
use crate::settings::pages::colors::{ACCENTS, SCHEMES};
use crate::theme::Theme;

const GAP: f32 = 10.0;
const ITEM_GAP: f32 = 6.0;

const CARD_HEIGHT: f32 = 38.0;
const CARD_PADDING: f32 = 8.0;
const ICON_SIZE: f32 = 28.0;

const HEADER_PADDING: f32 = 10.0;
const HEADER_ICON_SIZE: f32 = 36.0;

const SUBTAB_HEIGHT: f32 = 30.0;

const CHECK: &str = "󰄬";
const PALETTE_ICON: &str = "󰏘";
const CURSOR_ICON: &str = "󰍽";
const GLOW_ICON: &str = "󰵛";
const FONT_ICON: &str = "󰛄";
const CHEVRON_DOWN: &str = "󰅀";
const CHEVRON_UP: &str = "󰅃";

pub fn view(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Column {
    let title = title(theme, width);
    let subtabs = subtabs(overlay, theme, width);

    let available_height = (height - header::HEIGHT - SUBTAB_HEIGHT - GAP * 2.0).max(0.0);

    let (content, scroll_id) = match overlay.appearance_tab {
        0 => (colors_view(overlay, theme, width), "appearance:colors"),
        1 => (cursor_view(overlay, theme, width), "appearance:cursor"),
        _ => (fonts_view(overlay, theme, width), "appearance:fonts"),
    };

    let area = Rectangle::new()
        .width(width)
        .height(available_height)
        .child(ScrollArea::new(scroll_id, content));

    Column::new(children![title, subtabs, area]).gap(GAP)
}

fn title(theme: &Theme, width: f32) -> Rectangle {
    let icon = Rectangle::new()
        .width(HEADER_ICON_SIZE)
        .height(HEADER_ICON_SIZE)
        .radius(12.0)
        .fill(theme.accent)
        .align_child(Center, Center)
        .child(
            Text::new(PALETTE_ICON)
                .size(18.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.on_accent),
        );

    let text_width = width - HEADER_PADDING * 3.0 - HEADER_ICON_SIZE;

    let title = Text::new("Appearance")
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let status = Rectangle::new()
        .width(text_width)
        .height(16.0)
        .child(
            Text::new("Colors · Mouse Cursor · Fonts")
                .size(10.0)
                .font(fonts::BODY)
                .color(theme.muted_text)
                .elide(),
        );

    let text = Rectangle::new()
        .width(text_width)
        .height(HEADER_ICON_SIZE)
        .align_child(Start, Center)
        .child(Column::new(children![title, status]));

    Rectangle::new()
        .width(width)
        .height(header::HEIGHT)
        .radius(16.0)
        .fill(theme.surface)
        .padding(Padding {
            top: HEADER_PADDING,
            right: HEADER_PADDING,
            bottom: HEADER_PADDING,
            left: HEADER_PADDING,
        })
        .child(Row::new(children![icon, text]).gap(HEADER_PADDING).align(Center))
}

fn subtabs(overlay: &Overlay, theme: &Theme, width: f32) -> Row {
    let tabs = [
        (PALETTE_ICON, "Colors"),
        (CURSOR_ICON, "Cursor"),
        (FONT_ICON, "Fonts"),
    ];

    let count = tabs.len() as f32;
    let gap = 6.0;
    let button_width = (width - gap * (count - 1.0)) / count;

    let mut buttons: Vec<Box<dyn Widget>> = Vec::new();

    for (i, (icon, label)) in tabs.iter().enumerate() {
        let active = overlay.appearance_tab == i;
        let hover_name = format!("appearance:subtab:{i}");

        let fill = if active {
            theme.accent
        } else if hovered(overlay, &hover_name) {
            theme.hover_surface
        } else {
            theme.surface
        };

        let text_color = if active { theme.on_accent } else { theme.text };
        let icon_color = if active { theme.on_accent } else { theme.accent };

        let content = Row::new(children![
            Text::new(*icon).size(13.0).font(fonts::NERD).tight().color(icon_color),
            Text::new(*label)
                .size(11.0)
                .font(fonts::BODY)
                .weight(Weight::SemiBold)
                .color(text_color),
        ])
        .gap(6.0)
        .justify(Center)
        .align(Center);

        let button = Rectangle::new()
            .width(button_width)
            .height(SUBTAB_HEIGHT)
            .radius(SUBTAB_HEIGHT / 2.0)
            .fill(fill)
            .cursor(Pointer)
            .on_hover(move |inside| hover(hover_name.clone(), inside))
            .on_click(move |_| Overlay::write().appearance_tab = i)
            .align_child(Center, Center)
            .child(content);

        buttons.push(Box::new(button));
    }

    Row::new(buttons).width(width).height(SUBTAB_HEIGHT).gap(gap).justify(SpaceBetween).align(Center)
}

fn section_label(text: &'static str, theme: &Theme, width: f32) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(18.0)
        .padding(Padding {
            top: 2.0,
            right: 4.0,
            bottom: 0.0,
            left: 4.0,
        })
        .child(
            Text::new(text)
                .size(11.0)
                .font(fonts::BODY)
                .weight(Weight::SemiBold)
                .color(theme.secondary_text),
        )
}

fn scheme_label(scheme_id: &str) -> &'static str {
    for (label, id) in SCHEMES {
        if id == scheme_id {
            return label;
        }
    }
    SCHEMES[0].0
}

fn colors_view(overlay: &Overlay, theme: &Theme, width: f32) -> Column {
    let settings = Settings::read();
    let current_mode = settings.text("color_mode");
    let current_scheme = settings.text("scheme");
    let current_accent = settings.text("accent");
    let manual_accent = settings.flag("manual_accent");
    let current_transition = settings.text("wallpaper_transition");
    let current_speed = settings.number("animation_speed").max(0.1);

    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    items.push(Box::new(section_label("Color mode", theme, width)));
    items.push(Box::new(mode_row(overlay, theme, current_mode, width)));

    items.push(Box::new(section_label("Accent color", theme, width)));
    items.push(Box::new(wallpaper_accent_button(overlay, theme, manual_accent, width)));
    items.push(Box::new(accent_swatches_row(overlay, theme, manual_accent, current_accent, width)));

    items.push(Box::new(section_label("Color scheme", theme, width)));
    for widget in scheme_dropdown(overlay, theme, current_scheme, width) {
        items.push(widget);
    }

    items.push(Box::new(section_label("Wallpaper transition", theme, width)));
    items.push(Box::new(transition_row(overlay, theme, current_transition, width)));

    items.push(Box::new(section_label("Animation speed", theme, width)));
    items.push(Box::new(animation_speed_row(overlay, theme, current_speed, width)));

    Column::new(items).width(width).gap(ITEM_GAP)
}

fn mode_row(overlay: &Overlay, theme: &Theme, current_mode: &str, width: f32) -> Row {
    let modes = [("Auto", "auto"), ("Light", "light"), ("Dark", "dark")];
    let count = modes.len() as f32;
    let gap = 6.0;
    let pill_width = (width - gap * (count - 1.0)) / count;

    let mut pills: Vec<Box<dyn Widget>> = Vec::new();

    for (label, mode) in modes {
        let active = current_mode == mode;
        let hover_name = format!("appearance:mode:{mode}");

        let fill = if active {
            theme.accent
        } else if hovered(overlay, &hover_name) {
            theme.hover_surface
        } else {
            theme.surface
        };

        let text_color = if active { theme.on_accent } else { theme.text };

        let pill = Rectangle::new()
            .width(pill_width)
            .height(28.0)
            .radius(14.0)
            .fill(fill)
            .cursor(Pointer)
            .on_hover(move |inside| hover(hover_name.clone(), inside))
            .on_click(move |_| Settings::set("color_mode", mode))
            .align_child(Center, Center)
            .child(
                Text::new(label)
                    .size(11.0)
                    .font(fonts::BODY)
                    .weight(if active { Weight::SemiBold } else { Weight::Regular })
                    .color(text_color),
            );

        pills.push(Box::new(pill));
    }

    Row::new(pills).width(width).height(28.0).gap(gap).justify(SpaceBetween).align(Center)
}

fn animation_speed_row(overlay: &Overlay, theme: &Theme, current_speed: f32, width: f32) -> Row {
    let speeds: [(&str, &str, f32); 4] = [
        ("0.5×", "0.5", 0.5),
        ("1.0×", "1", 1.0),
        ("1.5×", "1.5", 1.5),
        ("2.0×", "2", 2.0),
    ];
    let count = speeds.len() as f32;
    let gap = 6.0;
    let pill_width = (width - gap * (count - 1.0)) / count;

    let mut pills: Vec<Box<dyn Widget>> = Vec::new();

    for (label, mode, val) in speeds {
        let active = (current_speed - val).abs() < 0.15;
        let hover_name = format!("appearance:speed:{mode}");

        let fill = if active {
            theme.accent
        } else if hovered(overlay, &hover_name) {
            theme.hover_surface
        } else {
            theme.surface
        };

        let text_color = if active { theme.on_accent } else { theme.text };

        let pill = Rectangle::new()
            .width(pill_width)
            .height(28.0)
            .radius(14.0)
            .fill(fill)
            .cursor(Pointer)
            .on_hover(move |inside| hover(hover_name.clone(), inside))
            .on_click(move |_| Settings::set("animation_speed", mode))
            .align_child(Center, Center)
            .child(
                Text::new(label)
                    .size(11.0)
                    .font(fonts::BODY)
                    .weight(if active { Weight::SemiBold } else { Weight::Regular })
                    .color(text_color),
            );

        pills.push(Box::new(pill));
    }

    Row::new(pills).width(width).height(28.0).gap(gap).justify(SpaceBetween).align(Center)
}


fn wallpaper_accent_button(overlay: &Overlay, theme: &Theme, manual_accent: bool, width: f32) -> Rectangle {
    let active = !manual_accent;
    let hover_name = String::from("appearance:accent:dynamic");

    let fill = if active {
        theme.selected_surface
    } else if hovered(overlay, &hover_name) {
        theme.hover_surface
    } else {
        theme.surface
    };

    let check = if active { CHECK } else { "" };

    let content = Row::new(children![
        Text::new(PALETTE_ICON).size(14.0).font(fonts::NERD).tight().color(theme.accent),
        Rectangle::new()
            .width(width - 70.0)
            .height(28.0)
            .align_child(Start, Center)
            .child(
                Text::new("Dynamic (Wallpaper accent)")
                    .size(11.0)
                    .font(fonts::BODY)
                    .weight(if active { Weight::SemiBold } else { Weight::Regular })
                    .color(theme.text),
            ),
        Text::new(check).size(13.0).font(fonts::NERD).tight().color(theme.accent),
    ])
    .gap(8.0)
    .align(Center);

    Rectangle::new()
        .width(width)
        .height(32.0)
        .radius(10.0)
        .fill(fill)
        .cursor(Pointer)
        .padding(Padding {
            top: 2.0,
            right: 12.0,
            bottom: 2.0,
            left: 10.0,
        })
        .on_hover(move |inside| hover(hover_name.clone(), inside))
        .on_click(|_| Settings::set("manual_accent", "false"))
        .align_child(Start, Center)
        .child(content)
}

fn accent_swatches_row(
    _overlay: &Overlay,
    theme: &Theme,
    manual_accent: bool,
    current_accent: &str,
    width: f32,
) -> Row {
    let count = ACCENTS.len() as f32;
    let size = 32.0;
    let gap = (width - count * size) / (count - 1.0);

    let mut swatches: Vec<Box<dyn Widget>> = Vec::new();

    for (name, hex) in ACCENTS {
        let is_selected = manual_accent && current_accent.eq_ignore_ascii_case(hex);
        let hover_name = format!("appearance:accent:{name}");

        let color = Color::from(hex);
        let check = if is_selected { CHECK } else { "" };

        let swatch = Rectangle::new()
            .width(size)
            .height(size)
            .radius(size / 2.0)
            .fill(color)
            .cursor(Pointer)
            .border(if is_selected { 2.0 } else { 1.0 }, if is_selected { theme.text } else { theme.border })
            .on_hover(move |inside| hover(hover_name.clone(), inside))
            .on_click(move |_| {
                Settings::set("manual_accent", "true");
                Settings::set("accent", hex);
            })
            .align_child(Center, Center)
            .child(
                Text::new(check)
                    .size(14.0)
                    .font(fonts::NERD)
                    .tight()
                    .color(Color::rgb(0x11, 0x11, 0x1b)),
            );

        swatches.push(Box::new(swatch));
    }

    Row::new(swatches).width(width).height(size).gap(gap).align(Center)
}

fn scheme_dropdown(
    overlay: &Overlay,
    theme: &Theme,
    current_scheme: &str,
    width: f32,
) -> Vec<Box<dyn Widget>> {
    let mut widgets: Vec<Box<dyn Widget>> = Vec::new();
    let is_open = overlay.scheme_dropdown_open;
    let label = scheme_label(current_scheme);
    let chevron = if is_open { CHEVRON_UP } else { CHEVRON_DOWN };
    let hover_name = String::from("appearance:scheme_dropdown_header");

    let fill = if is_open {
        theme.selected_surface
    } else if hovered(overlay, &hover_name) {
        theme.hover_surface
    } else {
        theme.surface
    };

    let text_width = width - CARD_PADDING * 2.0 - ICON_SIZE - 28.0;

    let header_content = Row::new(children![
        Rectangle::new()
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .radius(8.0)
            .fill(if is_open { theme.accent } else { theme.selected_surface })
            .align_child(Center, Center)
            .child(
                Text::new(PALETTE_ICON)
                    .size(14.0)
                    .font(fonts::NERD)
                    .tight()
                    .color(if is_open { theme.on_accent } else { theme.accent }),
            ),
        Rectangle::new()
            .width(text_width)
            .height(ICON_SIZE)
            .align_child(Start, Center)
            .child(
                Text::new(label)
                    .size(12.0)
                    .font(fonts::BODY)
                    .weight(Weight::SemiBold)
                    .color(theme.text),
            ),
        Text::new(chevron)
            .size(14.0)
            .font(fonts::NERD)
            .tight()
            .color(if is_open { theme.accent } else { theme.secondary_text }),
    ])
    .gap(CARD_PADDING)
    .align(Center);

    let mut header = Rectangle::new()
        .width(width)
        .height(CARD_HEIGHT)
        .radius(10.0)
        .fill(fill)
        .cursor(Pointer)
        .padding(Padding {
            top: CARD_PADDING,
            right: 12.0,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .on_hover(move |inside| hover(hover_name.clone(), inside))
        .on_click(|_| {
            let mut o = Overlay::write();
            o.scheme_dropdown_open = !o.scheme_dropdown_open;
        })
        .align_child(Start, Center)
        .child(header_content);

    if is_open {
        header = header.border(1.5, theme.accent);
    }

    widgets.push(Box::new(header));

    if is_open {
        let mut dropdown_items: Vec<Box<dyn Widget>> = Vec::new();
        let item_width = width - 16.0;

        for (item_label, id) in SCHEMES {
            let is_selected = current_scheme == id;
            let item_hover = format!("appearance:scheme_item:{id}");

            let item_fill = if is_selected {
                theme.selected_surface
            } else if hovered(overlay, &item_hover) {
                theme.hover_surface
            } else {
                Color::TRANSPARENT
            };

            let item_text_color = if is_selected {
                theme.accent
            } else {
                theme.text
            };

            let check_str = if is_selected { CHECK } else { "" };
            let label_width = item_width - 28.0;

            let row_content = Row::new(children![
                Rectangle::new()
                    .width(label_width)
                    .height(28.0)
                    .align_child(Start, Center)
                    .child(
                        Text::new(item_label)
                            .size(11.0)
                            .font(fonts::BODY)
                            .weight(if is_selected { Weight::SemiBold } else { Weight::Regular })
                            .color(item_text_color),
                    ),
                Text::new(check_str)
                    .size(13.0)
                    .font(fonts::NERD)
                    .tight()
                    .color(theme.accent),
            ])
            .align(Center);

            let item = Rectangle::new()
                .width(item_width)
                .height(30.0)
                .radius(8.0)
                .fill(item_fill)
                .cursor(Pointer)
                .padding(Padding {
                    top: 2.0,
                    right: 8.0,
                    bottom: 2.0,
                    left: 10.0,
                })
                .on_hover(move |inside| hover(item_hover.clone(), inside))
                .on_click(move |_| {
                    Settings::set("scheme", id);
                    Overlay::write().scheme_dropdown_open = false;
                })
                .align_child(Start, Center)
                .child(row_content);

            dropdown_items.push(Box::new(item));
        }

        let container_height = dropdown_items.len() as f32 * 30.0
            + (dropdown_items.len().saturating_sub(1)) as f32 * 2.0
            + 12.0;

        let container = Rectangle::new()
            .width(width)
            .height(container_height)
            .radius(10.0)
            .fill(theme.surface)
            .border(1.0, theme.border)
            .padding(Padding {
                top: 6.0,
                right: 8.0,
                bottom: 6.0,
                left: 8.0,
            })
            .child(Column::new(dropdown_items).width(item_width).gap(2.0));

        widgets.push(Box::new(container));
    }

    widgets
}

fn transition_row(overlay: &Overlay, theme: &Theme, current_transition: &str, width: f32) -> Row {
    let transitions = [("Circle", "circle"), ("Fade", "fade"), ("Instant", "instant")];
    let count = transitions.len() as f32;
    let gap = 6.0;
    let pill_width = (width - gap * (count - 1.0)) / count;

    let mut pills: Vec<Box<dyn Widget>> = Vec::new();

    for (label, mode) in transitions {
        let active = current_transition == mode;
        let hover_name = format!("appearance:transition:{mode}");

        let fill = if active {
            theme.accent
        } else if hovered(overlay, &hover_name) {
            theme.hover_surface
        } else {
            theme.surface
        };

        let text_color = if active { theme.on_accent } else { theme.text };

        let pill = Rectangle::new()
            .width(pill_width)
            .height(28.0)
            .radius(14.0)
            .fill(fill)
            .cursor(Pointer)
            .on_hover(move |inside| hover(hover_name.clone(), inside))
            .on_click(move |_| Settings::set("wallpaper_transition", mode))
            .align_child(Center, Center)
            .child(
                Text::new(label)
                    .size(11.0)
                    .font(fonts::BODY)
                    .weight(if active { Weight::SemiBold } else { Weight::Regular })
                    .color(text_color),
            );

        pills.push(Box::new(pill));
    }

    Row::new(pills).width(width).height(28.0).gap(gap).justify(SpaceBetween).align(Center)
}

fn cursor_view(overlay: &Overlay, theme: &Theme, width: f32) -> Column {
    let settings = Settings::read();
    let current_cursor = settings.text("cursor_theme");
    let current_size = settings.text("cursor_size");
    let shake_enabled = settings.flag("shake_cursor");
    let glow_enabled = settings.flag("cursor_glow");
    let glow_mode = settings.text("cursor_glow_mode");
    let glow_color = settings.text("cursor_glow_color");

    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    items.push(Box::new(section_label("Cursor size", theme, width)));
    items.push(Box::new(size_row(overlay, theme, current_size, width)));

    items.push(Box::new(section_label("Cursor glow", theme, width)));
    items.push(Box::new(glow_card(overlay, theme, glow_enabled, width)));
    if glow_enabled {
        items.push(Box::new(glow_mode_row(overlay, theme, glow_mode, width)));
        if glow_mode == "custom" {
            items.push(Box::new(glow_colors_row(overlay, theme, glow_color, width)));
        }
    }

    items.push(Box::new(section_label("Shake gesture", theme, width)));
    items.push(Box::new(shake_card(overlay, theme, shake_enabled, width)));

    items.push(Box::new(section_label("Cursor theme (10 popular)", theme, width)));
    for (label, id) in CURSORS {
        let is_selected = current_cursor == id;
        items.push(Box::new(cursor_card(overlay, theme, label, id, is_selected, width)));
    }

    Column::new(items).width(width).gap(ITEM_GAP)
}

fn glow_card(overlay: &Overlay, theme: &Theme, enabled: bool, width: f32) -> Rectangle {
    let hover_name = String::from("appearance:glow");

    let fill = if hovered(overlay, &hover_name) {
        theme.hover_surface
    } else {
        theme.surface
    };

    let switch_fill = if enabled { theme.accent } else { theme.border };
    let status_text = if enabled { "Enabled" } else { "Disabled" };

    let text_width = width - CARD_PADDING * 2.0 - ICON_SIZE - 60.0;

    let content = Row::new(children![
        Rectangle::new()
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .radius(8.0)
            .fill(theme.selected_surface)
            .align_child(Center, Center)
            .child(Text::new(GLOW_ICON).size(14.0).font(fonts::NERD).tight().color(theme.accent)),
        Rectangle::new()
            .width(text_width)
            .height(ICON_SIZE)
            .align_child(Start, Center)
            .child(
                Column::new(children![
                    Text::new("Cursor glow halo")
                        .size(11.0)
                        .font(fonts::BODY)
                        .weight(Weight::SemiBold)
                        .color(theme.text),
                    Text::new("Luminous colored halo radiating around cursor")
                        .size(9.0)
                        .font(fonts::BODY)
                        .color(theme.secondary_text),
                ])
                .gap(1.0),
            ),
        Rectangle::new()
            .width(50.0)
            .height(20.0)
            .radius(10.0)
            .fill(switch_fill)
            .align_child(Center, Center)
            .child(
                Text::new(status_text)
                    .size(9.0)
                    .font(fonts::BODY)
                    .weight(Weight::SemiBold)
                    .color(if enabled { theme.on_accent } else { theme.muted_text }),
            ),
    ])
    .gap(10.0)
    .align(Center);

    Rectangle::new()
        .width(width)
        .height(CARD_HEIGHT)
        .radius(10.0)
        .fill(fill)
        .cursor(Pointer)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .on_hover(move |inside| hover(hover_name.clone(), inside))
        .on_click(move |_| Settings::set("cursor_glow", !enabled))
        .child(content)
}

fn glow_mode_row(overlay: &Overlay, theme: &Theme, current_mode: &str, width: f32) -> Row {
    let modes = [("Match theme", "theme"), ("Pick a color", "custom")];
    let count = modes.len() as f32;
    let gap = 6.0;
    let pill_width = (width - gap * (count - 1.0)) / count;

    let mut pills: Vec<Box<dyn Widget>> = Vec::new();

    for (label, mode) in modes {
        let active = current_mode == mode;
        let hover_name = format!("appearance:glow_mode:{mode}");

        let fill = if active {
            theme.accent
        } else if hovered(overlay, &hover_name) {
            theme.hover_surface
        } else {
            theme.surface
        };

        let text_color = if active { theme.on_accent } else { theme.text };

        let pill = Rectangle::new()
            .width(pill_width)
            .height(28.0)
            .radius(14.0)
            .fill(fill)
            .cursor(Pointer)
            .on_hover(move |inside| hover(hover_name.clone(), inside))
            .on_click(move |_| Settings::set("cursor_glow_mode", mode))
            .align_child(Center, Center)
            .child(
                Text::new(label)
                    .size(11.0)
                    .font(fonts::BODY)
                    .weight(if active { Weight::SemiBold } else { Weight::Regular })
                    .color(text_color),
            );

        pills.push(Box::new(pill));
    }

    Row::new(pills).width(width).height(28.0).gap(gap).justify(SpaceBetween).align(Center)
}

fn glow_colors_row(overlay: &Overlay, theme: &Theme, current_color: &str, width: f32) -> Row {
    let colors = [
        ("#00d2ff", "Cyan"),
        ("#3b82f6", "Blue"),
        ("#ff2a85", "Pink"),
        ("#a855f7", "Purple"),
        ("#10b981", "Green"),
        ("#f59e0b", "Gold"),
        ("#ef4444", "Red"),
        ("#f8fafc", "White"),
    ];

    let count = colors.len() as f32;
    let gap = 5.0;
    let swatch_size = ((width - gap * (count - 1.0)) / count).floor();

    let mut swatches: Vec<Box<dyn Widget>> = Vec::new();

    for (hex, _name) in colors {
        let active = current_color.eq_ignore_ascii_case(hex);
        let hover_name = format!("appearance:glow_col:{hex}");
        let (r, g, b) = crate::cursor_glow::parse_hex_color(hex);
        let swatch_color = amane::Color::rgb(r, g, b);

        let border_color = if active {
            theme.text
        } else if hovered(overlay, &hover_name) {
            theme.secondary_text
        } else {
            theme.border
        };

        let border_width = if active { 2.5 } else { 1.0 };

        let swatch = Rectangle::new()
            .width(swatch_size)
            .height(26.0)
            .radius(6.0)
            .fill(swatch_color)
            .border(border_width, border_color)
            .cursor(Pointer)
            .on_hover(move |inside| hover(hover_name.clone(), inside))
            .on_click(move |_| Settings::set("cursor_glow_color", hex));

        swatches.push(Box::new(swatch));
    }

    Row::new(swatches).width(width).height(26.0).gap(gap).align(Center)
}

fn size_row(overlay: &Overlay, theme: &Theme, current_size: &str, width: f32) -> Row {
    let count = SIZES.len() as f32;
    let gap = 6.0;
    let pill_width = (width - gap * (count - 1.0)) / count;

    let mut pills: Vec<Box<dyn Widget>> = Vec::new();

    for (_label, size) in SIZES {
        let active = current_size == size;
        let hover_name = format!("appearance:size:{size}");

        let fill = if active {
            theme.accent
        } else if hovered(overlay, &hover_name) {
            theme.hover_surface
        } else {
            theme.surface
        };

        let text_color = if active { theme.on_accent } else { theme.text };
        let display = format!("{size}px");

        let pill = Rectangle::new()
            .width(pill_width)
            .height(28.0)
            .radius(14.0)
            .fill(fill)
            .cursor(Pointer)
            .on_hover(move |inside| hover(hover_name.clone(), inside))
            .on_click(move |_| Settings::set("cursor_size", size))
            .align_child(Center, Center)
            .child(
                Text::new(display)
                    .size(11.0)
                    .font(fonts::BODY)
                    .weight(if active { Weight::SemiBold } else { Weight::Regular })
                    .color(text_color),
            );

        pills.push(Box::new(pill));
    }

    Row::new(pills).width(width).height(28.0).gap(gap).justify(SpaceBetween).align(Center)
}

fn shake_card(overlay: &Overlay, theme: &Theme, enabled: bool, width: f32) -> Rectangle {
    let hover_name = String::from("appearance:shake");

    let fill = if hovered(overlay, &hover_name) {
        theme.hover_surface
    } else {
        theme.surface
    };

    let switch_fill = if enabled { theme.accent } else { theme.border };
    let status_text = if enabled { "Enabled" } else { "Disabled" };

    let text_width = width - CARD_PADDING * 2.0 - ICON_SIZE - 60.0;

    let content = Row::new(children![
        Rectangle::new()
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .radius(8.0)
            .fill(theme.selected_surface)
            .align_child(Center, Center)
            .child(Text::new(CURSOR_ICON).size(14.0).font(fonts::NERD).tight().color(theme.accent)),
        Rectangle::new()
            .width(text_width)
            .height(ICON_SIZE)
            .align_child(Start, Center)
            .child(
                Column::new(children![
                    Text::new("Shake to find cursor")
                        .size(11.0)
                        .font(fonts::BODY)
                        .weight(Weight::SemiBold)
                        .color(theme.text),
                    Text::new("Temporarily enlarges pointer")
                        .size(9.0)
                        .font(fonts::BODY)
                        .color(theme.muted_text),
                ]),
            ),
        Rectangle::new()
            .width(48.0)
            .height(22.0)
            .radius(11.0)
            .fill(switch_fill)
            .align_child(Center, Center)
            .child(
                Text::new(status_text)
                    .size(9.0)
                    .font(fonts::BODY)
                    .weight(Weight::SemiBold)
                    .color(if enabled { theme.on_accent } else { theme.text }),
            ),
    ])
    .gap(CARD_PADDING)
    .align(Center);

    Rectangle::new()
        .width(width)
        .height(CARD_HEIGHT)
        .radius(10.0)
        .fill(fill)
        .cursor(Pointer)
        .padding(Padding {
            top: CARD_PADDING,
            right: 10.0,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .on_hover(move |inside| hover(hover_name.clone(), inside))
        .on_click(move |_| Settings::set("shake_cursor", if enabled { "false" } else { "true" }))
        .align_child(Start, Center)
        .child(content)
}

fn cursor_card(
    overlay: &Overlay,
    theme: &Theme,
    label: &'static str,
    id: &'static str,
    selected: bool,
    width: f32,
) -> Rectangle {
    let hover_name = format!("appearance:cursor:{id}");

    let fill = if selected {
        theme.selected_surface
    } else if hovered(overlay, &hover_name) {
        theme.hover_surface
    } else {
        theme.surface
    };

    let weight = if selected { Weight::SemiBold } else { Weight::Regular };
    let check = if selected { CHECK } else { "" };

    let text_width = width - CARD_PADDING * 2.0 - ICON_SIZE - 24.0;

    let content = Row::new(children![
        Rectangle::new()
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .radius(8.0)
            .fill(theme.selected_surface)
            .align_child(Center, Center)
            .child(Text::new(CURSOR_ICON).size(14.0).font(fonts::NERD).tight().color(theme.accent)),
        Rectangle::new()
            .width(text_width)
            .height(ICON_SIZE)
            .align_child(Start, Center)
            .child(Text::new(label).size(11.0).font(fonts::BODY).weight(weight).color(theme.text)),
        Text::new(check).size(13.0).font(fonts::NERD).tight().color(theme.accent),
    ])
    .gap(CARD_PADDING)
    .align(Center);

    Rectangle::new()
        .width(width)
        .height(CARD_HEIGHT)
        .radius(10.0)
        .fill(fill)
        .cursor(Pointer)
        .padding(Padding {
            top: CARD_PADDING,
            right: 12.0,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .on_hover(move |inside| hover(hover_name.clone(), inside))
        .on_click(move |_| Settings::set("cursor_theme", id))
        .align_child(Start, Center)
        .child(content)
}

fn fonts_view(overlay: &Overlay, theme: &Theme, width: f32) -> Column {
    let settings = Settings::read();
    let current_font = settings.text("font_family");

    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    items.push(Box::new(section_label("Shell font family (10 options)", theme, width)));
    for (label, id) in FONTS {
        let is_selected = current_font == id;
        items.push(Box::new(font_card(overlay, theme, label, id, is_selected, width)));
    }

    Column::new(items).width(width).gap(ITEM_GAP)
}

fn font_card(
    overlay: &Overlay,
    theme: &Theme,
    label: &'static str,
    id: &'static str,
    selected: bool,
    width: f32,
) -> Rectangle {
    let hover_name = format!("appearance:font:{id}");

    let fill = if selected {
        theme.selected_surface
    } else if hovered(overlay, &hover_name) {
        theme.hover_surface
    } else {
        theme.surface
    };

    let weight = if selected { Weight::SemiBold } else { Weight::Regular };
    let check = if selected { CHECK } else { "" };

    let text_width = width - CARD_PADDING * 2.0 - ICON_SIZE - 24.0;

    let content = Row::new(children![
        Rectangle::new()
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .radius(8.0)
            .fill(theme.selected_surface)
            .align_child(Center, Center)
            .child(Text::new(FONT_ICON).size(14.0).font(fonts::NERD).tight().color(theme.accent)),
        Rectangle::new()
            .width(text_width)
            .height(ICON_SIZE)
            .align_child(Start, Center)
            .child(Text::new(label).size(11.0).font(fonts::BODY).weight(weight).color(theme.text)),
        Text::new(check).size(13.0).font(fonts::NERD).tight().color(theme.accent),
    ])
    .gap(CARD_PADDING)
    .align(Center);

    Rectangle::new()
        .width(width)
        .height(CARD_HEIGHT)
        .radius(10.0)
        .fill(fill)
        .cursor(Pointer)
        .padding(Padding {
            top: CARD_PADDING,
            right: 12.0,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .on_hover(move |inside| hover(hover_name.clone(), inside))
        .on_click(move |_| Settings::set("font_family", id))
        .align_child(Start, Center)
        .child(content)
}
