use amane::{
    Center, Column, Full, Padding, Pointer, Rectangle, Row, ScrollArea, Service,
    SpaceBetween, Start, Text, Weight, Widget, children,
};

use super::{Overlay, PanelView, Region};
use crate::bar::pill;
use crate::fonts;
use crate::liquid::Blob;
use crate::settings::Settings;
use crate::theme::Theme;

const WIDTH: f32 = 520.0;
const HEIGHT: f32 = 490.0;
const RADIUS: f32 = 24.0;

const TOP_OFFSET: f32 = 8.0;
const SHADOW_MARGIN: f32 = 20.0;

const PADDING: Padding = Padding {
    top: 16.0,
    right: 18.0,
    bottom: 16.0,
    left: 18.0,
};

const TAB_HEIGHT: f32 = 30.0;
const GAP: f32 = 12.0;

// Settings Pulldown View
pub fn view(overlay: &Overlay, theme: &Theme, screen: Region) -> Option<PanelView> {
    let progress = overlay.settings_menu.progress.value();

    if !overlay.settings_menu.shown && progress <= 0.001 {
        return None;
    }

    let width = WIDTH.min(screen.width - 24.0);
    let height = HEIGHT.min(screen.height - 24.0);

    let x = 12.0; // Left-aligned under the top-left pills

    let hidden = -(height + 32.0);
    let y = TOP_OFFSET + (hidden - TOP_OFFSET) * (1.0 - progress);

    let blob = Blob::new(x, y, width, height)
        .radius(RADIUS)
        .child(content(overlay, theme, width, height));

    let input = Region {
        x,
        y,
        width,
        height,
    };

    let reach = Region {
        x: (x - SHADOW_MARGIN).max(0.0),
        y: 0.0,
        width: width + SHADOW_MARGIN * 2.0,
        height: TOP_OFFSET + height + SHADOW_MARGIN,
    };

    Some(PanelView { blob, input, reach })
}

fn content(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = width - PADDING.left - PADDING.right;

    let header = header(theme, inner_width);
    let tabs = tab_bar(overlay, theme, inner_width);

    let available_height = (height - PADDING.top - PADDING.bottom - 44.0 - TAB_HEIGHT - GAP * 2.0).max(100.0);

    let page_content = match overlay.settings_tab {
        0 => appearance_tab(theme, inner_width),
        1 => bar_tab(theme, inner_width),
        2 => launcher_tab(theme, inner_width),
        3 => input_tab(theme, inner_width),
        _ => integrations_tab(theme, inner_width),
    };

    let scroll_id = match overlay.settings_tab {
        0 => "settings:appearance",
        1 => "settings:bar",
        2 => "settings:launcher",
        3 => "settings:input",
        _ => "settings:integrations",
    };

    let scroll_area = Rectangle::new()
        .width(inner_width)
        .height(available_height)
        .child(ScrollArea::new(scroll_id, page_content));

    Rectangle::new()
        .width(width)
        .height(height)
        .padding(PADDING)
        .child(Column::new(children![header, tabs, scroll_area]).gap(GAP))
}

fn header(theme: &Theme, width: f32) -> Row {
    let icon = Rectangle::new()
        .width(36.0)
        .height(36.0)
        .radius(10.0)
        .fill(theme.accent)
        .align_child(Center, Center)
        .child(
            Text::new("\u{f0493}")
                .size(18.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.on_accent),
        );

    let title_text = Text::new("Settings")
        .size(17.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let subtitle_text = Text::new("Quick shell & system preferences")
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::Light)
        .color(theme.secondary_text);

    let title_col = Column::new(children![title_text, subtitle_text]).gap(2.0);

    let left = Row::new(children![icon, title_col])
        .gap(10.0)
        .align(Center);

    let full_window_btn = Rectangle::new()
        .width(118.0)
        .height(28.0)
        .radius(Full)
        .fill(theme.surface)
        .cursor(Pointer)
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 0.0,
            left: 12.0,
        })
        .on_click(|_| {
            Overlay::toggle_settings_menu(None);
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
                    .color(theme.secondary_text)
            ])
            .align(Center),
        );

    Row::new(children![left, full_window_btn])
        .width(width)
        .justify(SpaceBetween)
        .align(Center)
}

fn tab_bar(overlay: &Overlay, theme: &Theme, width: f32) -> Row {
    let tabs = [
        ("󰏘 Appearance", 0),
        ("󰍹 Bar", 1),
        ("󰍉 Launcher", 2),
        ("󰌌 Input", 3),
        ("󱒓 Integrations", 4),
    ];

    let mut items: Vec<Box<dyn Widget>> = Vec::new();

    for (label, idx) in tabs {
        let active = overlay.settings_tab == idx;
        let fill = if active {
            theme.accent
        } else {
            theme.surface
        };
        let text_color = if active {
            theme.on_accent
        } else {
            theme.secondary_text
        };

        let pill_w = pill::text_width(label, 12.0) + 24.0;
        let tab_pill = Rectangle::new()
            .width(pill_w)
            .height(TAB_HEIGHT)
            .radius(Full)
            .fill(fill)
            .cursor(Pointer)
            .padding(Padding {
                top: 0.0,
                right: 12.0,
                bottom: 0.0,
                left: 12.0,
            })
            .on_click(move |_| Overlay::set_settings_tab(idx))
            .align_child(Center, Center)
            .child(
                Text::new(label)
                    .size(12.0)
                    .font(fonts::BODY)
                    .weight(if active { Weight::Medium } else { Weight::Regular })
                    .color(text_color),
            );

        items.push(Box::new(tab_pill));
    }

    Row::new(items)
        .width(width)
        .gap(6.0)
        .align(Center)
}

// ---------------- Tabs Content ----------------

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

    let power_row = row_switch(
        theme,
        width,
        "Power Button Pill",
        "Show power menu launcher on top right",
        settings.flag("bar_power"),
        "bar_power",
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

    Column::new(children![logo_row, tray_row, power_row, clock_24_row, seconds_row, dismiss_row]).gap(8.0)
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
        .weight(Weight::Medium)
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
            .fill(theme.hover_surface)
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
        .width(width - 24.0)
        .justify(SpaceBetween)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(48.0)
        .radius(14.0)
        .fill(theme.surface)
        .cursor(Pointer)
        .padding(Padding {
            top: 10.0,
            right: 12.0,
            bottom: 10.0,
            left: 12.0,
        })
        .on_click(move |_| {
            Settings::set(key, if checked { "false" } else { "true" });
        })
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
        .weight(Weight::Medium)
        .color(theme.text);

    let desc_text = Text::new(description)
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::Light)
        .color(theme.secondary_text);

    let text_col = Column::new(children![title_text, desc_text]).gap(2.0);

    let mut pills: Vec<Box<dyn Widget>> = Vec::new();
    for (label, selected, key, val) in options {
        let pill_w = pill::text_width(label, 11.0) + 20.0;
        let pill = Rectangle::new()
            .width(pill_w)
            .height(24.0)
            .radius(Full)
            .fill(if selected { theme.accent } else { theme.hover_surface })
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
                    .size(11.0)
                    .font(fonts::BODY)
                    .weight(if selected { Weight::Medium } else { Weight::Regular })
                    .color(if selected { theme.on_accent } else { theme.text }),
            );
        pills.push(Box::new(pill));
    }

    let pills_row = Row::new(pills).gap(4.0).align(Center);

    let card = Row::new(children![text_col, pills_row])
        .width(width - 24.0)
        .justify(SpaceBetween)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(48.0)
        .radius(14.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 10.0,
            right: 12.0,
            bottom: 10.0,
            left: 12.0,
        })
        .child(card)
}

// IPC Command Handler for Settings Menu
pub fn ipc(arguments: &[String]) -> String {
    let command = arguments.first().map(String::as_str).unwrap_or("toggle");
    let monitor_arg = arguments.get(1).map(String::as_str);

    match command {
        "show" => {
            let mut overlay = Overlay::write();
            overlay.set_active_monitor(monitor_arg);
            overlay.settings_menu.show();
        }
        "hide" => Overlay::write().settings_menu.hide(),
        _ => Overlay::toggle_settings_menu(monitor_arg),
    }

    String::from("ok")
}
