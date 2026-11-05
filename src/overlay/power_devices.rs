use std::process::Command;

use amane::{
    Center, Column, Padding, Pointer, Rectangle, Row,
    Service, SpaceBetween, Text, Weight, Widget, children,
};

use super::{Overlay, PanelView, Region};
use crate::fonts;
use crate::theme::Theme;

const CARD_PADDING: f32 = 16.0;
const CARD_RADIUS: f32 = 20.0;

#[derive(Clone, Debug, PartialEq)]
pub enum DeviceKind {
    Laptop,
    Keyboard,
    Mouse,
    Headphones,
    Other,
}

#[derive(Clone, Debug)]
pub struct PowerDevice {
    pub name: String,
    pub kind: DeviceKind,
    pub percent: u8,
    #[allow(dead_code)]
    pub state: String,
    pub is_charging: bool,
    pub details: String,
}

#[derive(Clone)]
pub struct PowerDevicesData {
    pub ac_online: bool,
    pub active_profile: String,
    pub laptop_battery: Option<PowerDevice>,
    pub peripherals: Vec<PowerDevice>,
}

impl Default for PowerDevicesData {
    fn default() -> Self {
        Self {
            ac_online: false,
            active_profile: String::from("balanced"),
            laptop_battery: None,
            peripherals: Vec::new(),
        }
    }
}

impl Service for PowerDevicesData {
    fn new() -> Self {
        let mut data = Self::default();
        data.update();
        data
    }

    fn interval() -> std::time::Duration {
        std::time::Duration::from_millis(2500)
    }

    fn update(&mut self) -> bool {
        let is_visible = {
            let overlay = Overlay::read();
            overlay.control_center.shown && overlay.center_tab == 4
        };

        if !is_visible && self.laptop_battery.is_some() {
            return false;
        }

        let (ac_online, laptop, peripherals) = query_power_devices();
        self.ac_online = ac_online;
        self.laptop_battery = laptop;
        self.peripherals = peripherals;
        self.active_profile = query_active_profile();
        true
    }
}

impl PowerDevicesData {
    pub fn set_profile(profile: &'static str) {
        Self::write().active_profile = profile.to_string();

        std::thread::spawn(move || {
            let _ = Command::new("busctl")
                .args([
                    "--system",
                    "set-property",
                    "net.hadess.PowerProfiles",
                    "/net/hadess/PowerProfiles",
                    "net.hadess.PowerProfiles",
                    "ActiveProfile",
                    "s",
                    profile,
                ])
                .status();
        });
    }
}

#[allow(dead_code)]
pub fn view(_overlay: &Overlay, _theme: &Theme, _screen: Region) -> Option<PanelView> {
    None
}

// Power timeout presets
const IDLE_PRESETS: [&str; 6] = ["Never", "1m", "2m", "5m", "10m", "15m"];
const DISPLAY_PRESETS: [&str; 6] = ["Never", "2m", "5m", "10m", "15m", "30m"];
const SLEEP_PRESETS: [&str; 6] = ["Never", "5m", "10m", "15m", "30m", "1h"];
const SUSPEND_PRESETS: [&str; 6] = ["Never", "15m", "30m", "45m", "1h", "2h"];

fn timeout_to_seconds(val: &str) -> u32 {
    match val {
        "1m" => 60,
        "2m" => 120,
        "5m" => 300,
        "10m" => 600,
        "15m" => 900,
        "30m" => 1800,
        "45m" => 2700,
        "1h" => 3600,
        "2h" => 7200,
        "3h" => 10800,
        _ => 0,
    }
}

pub fn init_timeouts() {
    apply_timeouts();
}

pub fn apply_timeouts() {
    let settings = crate::settings::Settings::read();
    let idle_str = settings.text("power_idle_timeout");
    let display_str = settings.text("power_display_timeout");
    let sleep_str = settings.text("power_sleep_timeout");
    let suspend_str = settings.text("power_suspend_timeout");

    let idle_secs = timeout_to_seconds(idle_str);
    let display_secs = timeout_to_seconds(display_str);
    let sleep_secs = timeout_to_seconds(sleep_str);
    let suspend_secs = timeout_to_seconds(suspend_str);

    // Save JSON state for any daemon or external tool
    if let Ok(state_dir) = std::env::var("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state")))
    {
        let amane_dir = state_dir.join("amane");
        let _ = std::fs::create_dir_all(&amane_dir);
        let path = amane_dir.join("power_timeouts.json");
        let json = format!(
            r#"{{"idle":"{}","idle_secs":{},"display":"{}","display_secs":{},"sleep":"{}","sleep_secs":{},"suspend":"{}","suspend_secs":{}}}"#,
            idle_str, idle_secs, display_str, display_secs, sleep_str, sleep_secs, suspend_str, suspend_secs
        );
        let _ = std::fs::write(path, json);
    }

    // Apply GNOME/Wayland session idle delay
    std::thread::spawn(move || {
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.session", "idle-delay", &idle_secs.to_string()])
            .status();

        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.screensaver", "lock-delay", "0"])
            .status();

        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.screensaver", "lock-enabled", if sleep_secs > 0 { "true" } else { "false" }])
            .status();
    });
}

fn step_preset(current: &str, presets: &[&'static str], step: i32) -> &'static str {
    let idx = presets.iter().position(|&p| p == current).unwrap_or(0);
    let new_idx = if step < 0 {
        idx.saturating_sub(1)
    } else {
        (idx + 1).min(presets.len() - 1)
    };
    presets[new_idx]
}

fn cycle_preset(current: &str, presets: &[&'static str]) -> &'static str {
    let idx = presets.iter().position(|&p| p == current).unwrap_or(0);
    presets[(idx + 1) % presets.len()]
}

fn timeout_column(
    theme: &Theme,
    title: &'static str,
    icon: &'static str,
    desc: &'static str,
    key: &'static str,
    presets: &'static [&'static str],
    width: f32,
    height: f32,
) -> Rectangle {
    let current_str = crate::settings::Settings::read().text(key).to_string();
    let is_never = current_str == "Never";

    let icon_w = Text::new(icon)
        .size(13.0)
        .font(fonts::NERD)
        .tight()
        .color(if is_never { theme.secondary_text } else { theme.accent });

    let title_w = Text::new(title)
        .size(11.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let desc_w = Text::new(desc)
        .size(10.0)
        .font(fonts::BODY)
        .color(theme.muted_text)
        .elide();

    let title_row = Row::new(children![icon_w, title_w, desc_w])
        .gap(4.0)
        .align(Center);

    let btn_size = 24.0;

    let cur_for_dec = current_str.clone();
    let btn_dec = Rectangle::new()
        .width(btn_size)
        .height(btn_size)
        .radius(6.0)
        .fill(theme.surface)
        .cursor(Pointer)
        .on_click(move |_| {
            let next = step_preset(&cur_for_dec, presets, -1);
            crate::settings::Settings::set(key, next);
            apply_timeouts();
        })
        .align_child(Center, Center)
        .child(
            Text::new("󰍴")
                .size(11.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.secondary_text),
        );

    let cur_for_inc = current_str.clone();
    let btn_inc = Rectangle::new()
        .width(btn_size)
        .height(btn_size)
        .radius(6.0)
        .fill(theme.surface)
        .cursor(Pointer)
        .on_click(move |_| {
            let next = step_preset(&cur_for_inc, presets, 1);
            crate::settings::Settings::set(key, next);
            apply_timeouts();
        })
        .align_child(Center, Center)
        .child(
            Text::new("󰐕")
                .size(11.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.secondary_text),
        );

    let cur_for_cycle = current_str.clone();
    let pill_w = (width - (btn_size * 2.0) - 16.0 - 10.0).max(46.0);
    let (pill_bg, pill_fg, pill_weight) = if is_never {
        (theme.surface, theme.secondary_text, Weight::Medium)
    } else {
        (theme.selected_surface, theme.accent, Weight::SemiBold)
    };

    let val_label = Text::new(&current_str)
        .size(11.5)
        .font(fonts::BODY)
        .weight(pill_weight)
        .color(pill_fg);

    let val_pill = Rectangle::new()
        .width(pill_w)
        .height(btn_size)
        .radius(6.0)
        .fill(pill_bg)
        .cursor(Pointer)
        .on_click(move |_| {
            let next = cycle_preset(&cur_for_cycle, presets);
            crate::settings::Settings::set(key, next);
            apply_timeouts();
        })
        .align_child(Center, Center)
        .child(val_label);

    let stepper_row = Row::new(children![btn_dec, val_pill, btn_inc])
        .gap(5.0)
        .align(Center);

    let col_content = Column::new(children![title_row, stepper_row])
        .gap(7.0)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(10.0)
        .fill(theme.selected_surface)
        .padding(Padding {
            top: 7.0,
            bottom: 7.0,
            left: 8.0,
            right: 8.0,
        })
        .align_child(Center, Center)
        .child(col_content)
}

fn timeouts_card(theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = width - CARD_PADDING * 2.0;

    let header_icon = Text::new("󱎫")
        .size(14.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.accent);

    let header_title = Text::new("Power & Sleep Timeouts")
        .size(12.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let header_left = Row::new(children![header_icon, header_title])
        .gap(6.0)
        .align(Center);

    let header_hint = Text::new("Automatic system dimming, display sleep, lock, and suspend")
        .size(11.0)
        .font(fonts::BODY)
        .color(theme.muted_text);

    let header_row = Row::new(children![header_left, header_hint])
        .width(inner_width)
        .justify(SpaceBetween)
        .align(Center);

    let gap = 8.0;
    let col_w = (inner_width - (gap * 3.0)) / 4.0;
    let col_h = 68.0;

    let col_idle = timeout_column(
        theme,
        "Idle",
        "󰒲",
        "Dim",
        "power_idle_timeout",
        &IDLE_PRESETS,
        col_w,
        col_h,
    );

    let col_display = timeout_column(
        theme,
        "Display",
        "󰍹",
        "Screen Off",
        "power_display_timeout",
        &DISPLAY_PRESETS,
        col_w,
        col_h,
    );

    let col_sleep = timeout_column(
        theme,
        "Sleep",
        "󰌾",
        "Lock",
        "power_sleep_timeout",
        &SLEEP_PRESETS,
        col_w,
        col_h,
    );

    let col_suspend = timeout_column(
        theme,
        "Suspend",
        "󰤄",
        "Standby",
        "power_suspend_timeout",
        &SUSPEND_PRESETS,
        col_w,
        col_h,
    );

    let cols_row = Row::new(children![col_idle, col_display, col_sleep, col_suspend])
        .width(inner_width)
        .gap(gap)
        .align(Center);

    let card_content = Column::new(children![header_row, cols_row])
        .gap(8.0);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: 10.0,
            right: CARD_PADDING,
            bottom: 10.0,
            left: CARD_PADDING,
        })
        .child(card_content)
}

// Layout rendered inside the middle pulldown's Power tab:
// Battery / Peripheral cards on top, Timeout settings in the middle, Power Menu actions on the bottom
pub fn embedded_view(theme: &Theme, width: f32, height: f32) -> Column {
    let gap = 10.0;
    let power_bar_h = 52.0;
    let timeouts_h = 118.0;
    let top_h = height - power_bar_h - timeouts_h - (gap * 2.0);

    let col_width = (width - gap) / 2.0;

    let left_card = laptop_card(theme, col_width, top_h);
    let right_card = peripherals_card(theme, col_width, top_h);

    let top_row = Row::new(children![left_card, right_card])
        .width(width)
        .height(top_h)
        .gap(gap)
        .align(Center);

    let timeouts = timeouts_card(theme, width, timeouts_h);
    let power_bar = power_actions_bar(theme, width, power_bar_h);

    Column::new(children![top_row, timeouts, power_bar])
        .width(width)
        .height(height)
        .gap(gap)
}

fn power_actions_bar(theme: &Theme, width: f32, height: f32) -> Row {
    let actions: [(&'static str, &'static str, &'static str, bool); 5] = [
        ("Lock", "\u{f033e}", "loginctl lock-session", false),
        ("Sleep", "\u{f04b2}", "systemctl suspend", false),
        ("Logout", "\u{f0343}", "niri msg action quit --skip-confirmation", false),
        ("Restart", "\u{f0709}", "systemctl reboot", false),
        ("Shutdown", "\u{23fb}", "systemctl poweroff", true),
    ];

    let gap = 10.0;
    let btn_w = (width - gap * 4.0) / 5.0;

    let mut buttons: Vec<Box<dyn Widget>> = Vec::new();
    for (label, icon, cmd, danger) in actions {
        buttons.push(Box::new(power_action_button(
            theme, label, icon, cmd, danger, btn_w, height,
        )));
    }

    Row::new(buttons)
        .width(width)
        .height(height)
        .gap(gap)
        .align(Center)
}

fn power_action_button(
    theme: &Theme,
    label: &'static str,
    icon: &'static str,
    command: &'static str,
    danger: bool,
    width: f32,
    height: f32,
) -> Rectangle {
    let icon_color = if danger { theme.danger } else { theme.accent };
    let text_color = if danger { theme.danger } else { theme.text };

    let icon_w = Text::new(icon)
        .size(16.0)
        .font(fonts::NERD)
        .tight()
        .color(icon_color);

    let label_w = Text::new(label)
        .size(12.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(text_color);

    let content = Row::new(children![icon_w, label_w])
        .gap(8.0)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(14.0)
        .fill(theme.surface)
        .cursor(Pointer)
        .on_click(move |_| {
            Overlay::write().control_center.hide();
            amane::spawn(command);
        })
        .align_child(Center, Center)
        .child(content)
}

fn laptop_card(theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = width - CARD_PADDING * 2.0;
    let data = PowerDevicesData::read();

    let (name, percent, is_charging, details) = match &data.laptop_battery {
        Some(b) => (b.name.as_str(), b.percent, b.is_charging, b.details.as_str()),
        None => ("Internal Battery", 100, false, "Not detected"),
    };

    let color = if is_charging {
        theme.success
    } else if percent > 20 {
        theme.accent
    } else {
        theme.danger
    };

    // Header row with AC status pill and power stats button
    let title = Text::new("Laptop Battery")
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let (ac_icon, ac_text, ac_color) = if data.ac_online {
        ("󱚥", "AC Connected", theme.success)
    } else {
        ("󰁹", "On Battery", theme.secondary_text)
    };

    let ac_icon_w = Text::new(ac_icon).size(12.0).font(fonts::NERD).tight().color(ac_color);
    let ac_text_w = Text::new(ac_text).size(11.0).font(fonts::BODY).weight(Weight::Medium).color(ac_color);

    let ac_w = ac_text.chars().count() as f32 * 6.8 + 30.0;
    let ac_pill = Rectangle::new()
        .width(ac_w)
        .height(24.0)
        .radius(8.0)
        .fill(theme.selected_surface)
        .padding(Padding {
            top: 2.0,
            bottom: 2.0,
            left: 8.0,
            right: 8.0,
        })
        .align_child(Center, Center)
        .child(Row::new(children![ac_icon_w, ac_text_w]).gap(4.0).align(Center));

    let details_btn = Rectangle::new()
        .width(26.0)
        .height(24.0)
        .radius(8.0)
        .fill(theme.selected_surface)
        .cursor(Pointer)
        .on_click(|_| {
            let _ = Command::new("gnome-power-statistics").spawn();
        })
        .align_child(Center, Center)
        .child(Text::new("󰊮").size(13.0).font(fonts::NERD).tight().color(theme.secondary_text));

    let header_right = Row::new(children![ac_pill, details_btn]).gap(6.0).align(Center);
    let header_row = Row::new(children![title, header_right])
        .width(inner_width)
        .justify(SpaceBetween)
        .align(Center);

    let battery_glyph = if is_charging {
        "󱚥"
    } else {
        match percent {
            90.. => "󰁹",
            70..=89 => "󰂁",
            50..=69 => "󰁿",
            30..=49 => "󰁽",
            15..=29 => "󰁻",
            _ => "󰁺",
        }
    };

    let icon_w = Text::new(battery_glyph).size(28.0).font(fonts::NERD).tight().color(color);
    let pct_w = Text::new(format!("{percent}%")).size(22.0).font(fonts::BODY).weight(Weight::Bold).color(theme.text);
    let left_stat = Row::new(children![icon_w, pct_w]).gap(8.0).align(Center);

    let sub_name = Text::new(name).size(11.0).font(fonts::BODY).color(theme.muted_text).elide();
    let detail_color = if is_charging { theme.success } else { theme.accent };
    let status_detail = Text::new(details).size(11.0).font(fonts::BODY).weight(Weight::Medium).color(detail_color);

    let right_info = Column::new(children![sub_name, status_detail]).gap(3.0);

    let stat_row = Row::new(children![left_stat, right_info])
        .width(inner_width)
        .justify(SpaceBetween)
        .align(Center);

    // Energy Bar
    let bar_height: f32 = 8.0;
    let track = Rectangle::new()
        .width(inner_width)
        .height(bar_height)
        .radius(bar_height / 2.0)
        .fill(theme.border);

    let fill_w = ((f32::from(percent) / 100.0) * inner_width).clamp(bar_height, inner_width);
    let filled = Rectangle::new()
        .width(fill_w)
        .height(bar_height)
        .radius(bar_height / 2.0)
        .fill(color);

    let bar = amane::Stack::new(children![track, filled]).width(inner_width).height(bar_height);

    let profile_label = Text::new("Power Profile")
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.secondary_text);

    let switcher = power_profile_switcher(theme, &data.active_profile, inner_width);
    let profile_section = Column::new(children![profile_label, switcher]).gap(6.0);

    let card_content = Column::new(children![header_row, stat_row, bar, profile_section]).gap(14.0);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .child(card_content)
}

fn power_profile_switcher(theme: &Theme, active_profile: &str, width: f32) -> Row {
    let btn_w = (width - 8.0) / 3.0;

    let p_saver = profile_button(
        theme,
        "power-saver",
        "Power Saver",
        "󰌪",
        active_profile == "power-saver",
        btn_w,
    );

    let p_balanced = profile_button(
        theme,
        "balanced",
        "Balanced",
        "󰾅",
        active_profile == "balanced",
        btn_w,
    );

    let p_perf = profile_button(
        theme,
        "performance",
        "Performance",
        "󱓅",
        active_profile == "performance",
        btn_w,
    );

    Row::new(children![p_saver, p_balanced, p_perf])
        .width(width)
        .gap(4.0)
        .align(Center)
}

fn profile_button(
    theme: &Theme,
    profile_id: &'static str,
    label: &str,
    icon: &str,
    is_active: bool,
    width: f32,
) -> Rectangle {
    let (fill, icon_color, text_color, weight) = if is_active {
        (theme.selected_surface, theme.accent, theme.text, Weight::SemiBold)
    } else {
        (amane::Color::TRANSPARENT, theme.secondary_text, theme.secondary_text, Weight::Medium)
    };

    let icon_w = Text::new(icon)
        .size(12.0)
        .font(fonts::NERD)
        .tight()
        .color(icon_color);

    let label_w = Text::new(label)
        .size(11.0)
        .font(fonts::BODY)
        .weight(weight)
        .color(text_color)
        .elide();

    let row = Row::new(children![icon_w, label_w])
        .gap(4.0)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(28.0)
        .radius(8.0)
        .fill(fill)
        .cursor(Pointer)
        .on_click(move |_| {
            PowerDevicesData::set_profile(profile_id);
        })
        .align_child(Center, Center)
        .child(row)
}

fn peripherals_card(theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = width - CARD_PADDING * 2.0;
    let data = PowerDevicesData::read();

    let header_icon = Text::new("󰂯").size(14.0).font(fonts::NERD).tight().color(theme.accent);
    let count_text = format!("Connected Devices ({})", data.peripherals.len());
    let header_title = Text::new(&count_text).size(13.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text);
    let header_row = Row::new(children![header_icon, header_title]).gap(6.0).align(Center);

    let mut items: Vec<Box<dyn Widget>> = Vec::new();
    items.push(Box::new(header_row));

    if data.peripherals.is_empty() {
        let empty_text = Text::new("No wireless peripherals connected")
            .size(12.0)
            .font(fonts::BODY)
            .color(theme.muted_text);

        let empty_row = Rectangle::new()
            .width(inner_width)
            .height(height - CARD_PADDING * 2.0 - 30.0)
            .align_child(Center, Center)
            .child(empty_text);

        items.push(Box::new(empty_row));
    } else {
        for dev in data.peripherals.iter().take(3) {
            items.push(Box::new(peripheral_row(theme, dev, inner_width)));
        }
    }

    let card_content = Column::new(items).gap(12.0);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .child(card_content)
}

fn peripheral_row(theme: &Theme, dev: &PowerDevice, inner_width: f32) -> Column {
    let icon_str = match dev.kind {
        DeviceKind::Keyboard => "󰌌",
        DeviceKind::Mouse => "󰍽",
        DeviceKind::Headphones => "󰋋",
        _ => "󰂯",
    };

    let dev_color = if dev.percent > 20 {
        theme.accent
    } else {
        theme.danger
    };

    let icon_w = Text::new(icon_str).size(16.0).font(fonts::NERD).tight().color(theme.accent);
    let name_w = Text::new(&dev.name).size(12.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text).elide();
    let detail_w = Text::new(&dev.details).size(10.0).font(fonts::BODY).color(theme.muted_text);

    let text_col = Column::new(children![name_w, detail_w]).gap(2.0);
    let left = Row::new(children![icon_w, text_col]).gap(8.0).align(Center);

    // Battery percentage pill on the right
    let batt_icon = Text::new("󰁹").size(10.0).font(fonts::NERD).tight().color(dev_color);
    let pct_label = Text::new(format!("{}%", dev.percent)).size(11.0).font(fonts::BODY).weight(Weight::SemiBold).color(dev_color);
    let pill_row = Row::new(children![batt_icon, pct_label]).gap(4.0).align(Center);

    let right_pill = Rectangle::new()
        .width(50.0)
        .height(22.0)
        .radius(6.0)
        .fill(theme.selected_surface)
        .padding(Padding {
            top: 2.0,
            bottom: 2.0,
            left: 6.0,
            right: 6.0,
        })
        .align_child(Center, Center)
        .child(pill_row);

    let top_line = Row::new(children![left, right_pill])
        .width(inner_width)
        .justify(SpaceBetween)
        .align(Center);

    // Mini battery track
    let bar_height: f32 = 4.0;
    let track = Rectangle::new()
        .width(inner_width)
        .height(bar_height)
        .radius(bar_height / 2.0)
        .fill(theme.border);

    let fill_w = ((f32::from(dev.percent) / 100.0) * inner_width).clamp(bar_height, inner_width);
    let filled = Rectangle::new()
        .width(fill_w)
        .height(bar_height)
        .radius(bar_height / 2.0)
        .fill(dev_color);

    let mini_bar = amane::Stack::new(children![track, filled]).width(inner_width).height(bar_height);

    Column::new(children![top_line, mini_bar]).gap(5.0)
}

fn query_power_devices() -> (bool, Option<PowerDevice>, Vec<PowerDevice>) {
    let Ok(output) = Command::new("upower").arg("-d").output() else {
        return (false, None, Vec::new());
    };

    let text = String::from_utf8_lossy(&output.stdout);

    let mut ac_online = false;
    let mut laptop_battery = None;
    let mut peripherals = Vec::new();

    let mut current_path = String::new();
    let mut model = String::new();
    let mut dev_type = String::new();
    let mut state = String::new();
    let mut percent: Option<u8> = None;
    let mut rate = String::new();
    let mut time_str = String::new();

    let flush_device = |path: &str,
                        model: &str,
                        dev_type: &str,
                        state: &str,
                        percent: Option<u8>,
                        rate: &str,
                        time_str: &str,
                        laptop_battery: &mut Option<PowerDevice>,
                        peripherals: &mut Vec<PowerDevice>| {
        let Some(p) = percent else { return };
        if path.contains("DisplayDevice") {
            return;
        }

        let is_charging = state.eq_ignore_ascii_case("charging");
        let name = if !model.is_empty() {
            model.to_string()
        } else if path.contains("battery_BAT") {
            String::from("Internal Battery")
        } else {
            String::from("Wireless Device")
        };

        if path.contains("battery_BAT") || dev_type == "battery" && laptop_battery.is_none() {
            let mut details = String::new();
            if !rate.is_empty() {
                details.push_str(rate);
            }
            if !time_str.is_empty() {
                if !details.is_empty() {
                    details.push_str(" • ");
                }
                details.push_str(time_str);
            }
            if details.is_empty() {
                details = state.to_string();
            }

            *laptop_battery = Some(PowerDevice {
                name,
                kind: DeviceKind::Laptop,
                percent: p,
                state: state.to_string(),
                is_charging,
                details,
            });
        } else {
            let kind = if dev_type == "keyboard"
                || path.contains("keyboard")
                || name.to_lowercase().contains("keyboard")
                || name.contains("BT-1")
            {
                DeviceKind::Keyboard
            } else if dev_type == "mouse"
                || path.contains("mouse")
                || name.to_lowercase().contains("mouse")
                || name.contains("M575")
            {
                DeviceKind::Mouse
            } else if dev_type == "headphones"
                || dev_type == "headset"
                || name.to_lowercase().contains("headphone")
                || name.to_lowercase().contains("buds")
                || name.to_lowercase().contains("airpods")
            {
                DeviceKind::Headphones
            } else {
                DeviceKind::Other
            };

            let details = match kind {
                DeviceKind::Keyboard => String::from("Wireless Keyboard"),
                DeviceKind::Mouse => String::from("Wireless Mouse"),
                DeviceKind::Headphones => String::from("Bluetooth Audio"),
                _ => String::from("Connected Peripheral"),
            };

            peripherals.push(PowerDevice {
                name,
                kind,
                percent: p,
                state: state.to_string(),
                is_charging,
                details,
            });
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if line.starts_with("Device: ") {
            flush_device(
                &current_path,
                &model,
                &dev_type,
                &state,
                percent,
                &rate,
                &time_str,
                &mut laptop_battery,
                &mut peripherals,
            );
            current_path = line.trim_start_matches("Device: ").trim().to_string();
            model.clear();
            dev_type.clear();
            state.clear();
            percent = None;
            rate.clear();
            time_str.clear();
        } else if trimmed.starts_with("model:") {
            if let Some((_, v)) = trimmed.split_once(':') {
                model = v.trim().to_string();
            }
        } else if trimmed.starts_with("state:") {
            if let Some((_, v)) = trimmed.split_once(':') {
                state = v.trim().to_string();
            }
        } else if trimmed.starts_with("percentage:") {
            if let Some((_, v)) = trimmed.split_once(':') {
                let s = v.trim().trim_end_matches('%');
                if let Ok(num) = s.parse::<f32>() {
                    percent = Some(num.round() as u8);
                }
            }
        } else if trimmed.starts_with("energy-rate:") {
            if let Some((_, v)) = trimmed.split_once(':') {
                rate = v.trim().to_string();
            }
        } else if trimmed.starts_with("time to full:") || trimmed.starts_with("time to empty:") {
            if let Some((_, v)) = trimmed.split_once(':') {
                time_str = v.trim().to_string();
            }
        } else if trimmed.starts_with("online:") {
            if let Some((_, v)) = trimmed.split_once(':') {
                if v.trim().eq_ignore_ascii_case("yes") {
                    ac_online = true;
                }
            }
        } else if matches!(trimmed, "battery" | "keyboard" | "mouse" | "headphones" | "headset") {
            dev_type = trimmed.to_string();
        }
    }

    flush_device(
        &current_path,
        &model,
        &dev_type,
        &state,
        percent,
        &rate,
        &time_str,
        &mut laptop_battery,
        &mut peripherals,
    );

    (ac_online, laptop_battery, peripherals)
}

pub fn ipc(arguments: &[String]) -> String {
    let monitor_arg = arguments.iter().find(|arg| !matches!(arg.as_str(), "toggle" | "show" | "hide")).map(String::as_str);
    Overlay::toggle_power_devices(monitor_arg);
    String::from("ok")
}

fn query_active_profile() -> String {
    let Ok(output) = Command::new("busctl")
        .args([
            "--system",
            "get-property",
            "net.hadess.PowerProfiles",
            "/net/hadess/PowerProfiles",
            "net.hadess.PowerProfiles",
            "ActiveProfile",
        ])
        .output()
    else {
        return String::from("balanced");
    };

    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace()
        .last()
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_else(|| String::from("balanced"))
}
