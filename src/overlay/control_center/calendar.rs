use std::fs;
use std::path::PathBuf;

use amane::{
    Center, Color, Column, End, Full, Notification, Notifications, Padding, Pointer, Rectangle, Row, ScrollArea, Service,
    SpaceBetween, Stack, Start, Text, TextInput, Weight, Widget, children,
};

use crate::clock::{self, Clock};
use crate::floating::Weather;
use crate::floating::weather::{ForecastDay, weather_code_icon};
use crate::fonts;
use crate::motion::{self, DEFAULT_SPATIAL};
use crate::overlay::Overlay;
use crate::theme::{self, Theme};

const PADDING: f32 = 14.0;
const ROWS: usize = 6;
const NAVIGATION_SIZE: f32 = 26.0;

const WEEKDAYS: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

const PREVIOUS_ICON: &str = "\u{f053}"; // Nerd Font chevron-left
const NEXT_ICON: &str = "\u{f054}";     // Nerd Font chevron-right

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ReminderItem {
    pub id: u64,
    pub tag: String,
    pub text: String,
}

#[derive(Clone, Default)]
pub struct RemindersData {
    pub items: Vec<ReminderItem>,
    pub input_text: String,
}

impl Service for RemindersData {
    fn new() -> Self {
        Self {
            items: load_reminders(),
            input_text: String::new(),
        }
    }

    // only changes through user input
    fn listen() {}
}

fn reminders_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/rcrout"));
    PathBuf::from(format!("{home}/.local/state/amane/reminders.json"))
}

fn load_reminders() -> Vec<ReminderItem> {
    let path = reminders_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(items) = serde_json::from_str::<Vec<ReminderItem>>(&content) {
            return items;
        }
    }

    vec![ReminderItem {
        id: 1,
        tag: String::from("all day"),
        text: String::from("This is a test"),
    }]
}

fn save_reminders(items: &[ReminderItem]) {
    let path = reminders_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(items) {
        let _ = fs::write(&path, json);
    }
}

pub fn add_reminder(input: &str) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return;
    }

    let (tag, desc) = if let Some((first, rest)) = trimmed.split_once(' ') {
        let first_lower = first.to_lowercase();
        if first_lower.ends_with("am") || first_lower.ends_with("pm") || first_lower == "all" {
            (first.to_string(), rest.trim().to_string())
        } else {
            (String::from("all day"), trimmed.to_string())
        }
    } else {
        (String::from("all day"), trimmed.to_string())
    };

    let mut r = RemindersData::write();
    let next_id = r.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    r.items.push(ReminderItem {
        id: next_id,
        tag,
        text: desc,
    });
    save_reminders(&r.items);
    r.input_text.clear();
    TextInput::set_text("new_reminder_input", "");
}

pub fn delete_reminder(id: u64) {
    let mut r = RemindersData::write();
    r.items.retain(|i| i.id != id);
    save_reminders(&r.items);
}

// Redesigned modern pulldown: Top Weather & 7-day forecast card + Bottom split (Clock, Reminders, Calendar)
pub fn embedded_view(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Column {
    let weather_card_height = 92.0;
    let gap = 12.0;
    let bottom_height = (height - weather_card_height - gap).max(100.0);

    let top_weather = weather_card(theme, width, weather_card_height);
    let bottom_cards = bottom_section(overlay, theme, width, bottom_height);

    Column::new(children![top_weather, bottom_cards]).gap(gap)
}

fn weather_card(theme: &Theme, width: f32, height: f32) -> Rectangle {
    let weather = Weather::read();

    let temp_str = match weather.temperature {
        Some(t) => format!("{t:.0}°F"),
        None => String::from("71°F"),
    };

    let cond_str = if weather.place.is_empty() && weather.temperature.is_none() {
        "Clear sky"
    } else {
        weather.condition()
    };

    let icon_str = if weather.temperature.is_none() {
        "\u{f0599}"
    } else {
        weather.icon()
    };

    let current_icon = Text::new(icon_str)
        .size(38.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.accent);

    let temp_label = Text::new(temp_str)
        .size(26.0)
        .font(fonts::BODY)
        .weight(Weight::Bold)
        .color(theme.text);

    let cond_label = Text::new(cond_str)
        .size(12.0)
        .font(fonts::BODY)
        .weight(Weight::Medium)
        .color(theme.secondary_text);

    let text_col = Column::new(children![temp_label, cond_label]).gap(2.0);

    let left_section = Row::new(children![current_icon, text_col])
        .gap(14.0)
        .align(Center);

    let left_box = Rectangle::new()
        .width(200.0)
        .height(height - 24.0)
        .align_child(Start, Center)
        .child(left_section);

    let separator = Rectangle::new()
        .width(1.0)
        .height(54.0)
        .fill(theme::with_opacity(theme.border, 0.6));

    let forecast_width = (width - 32.0 - 200.0 - 1.0 - 24.0).max(100.0);
    let forecast_row = forecast_strip(theme, &weather, forecast_width);

    let content = Row::new(children![left_box, separator, forecast_row])
        .gap(12.0)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(20.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 12.0,
            right: 16.0,
            bottom: 12.0,
            left: 16.0,
        })
        .align_child(Start, Center)
        .child(content)
}

fn forecast_strip(theme: &Theme, weather: &Weather, width: f32) -> Row {
    let default_days: Vec<ForecastDay> = get_default_forecast();
    let days_to_show = if weather.forecast_7d.len() >= 7 {
        &weather.forecast_7d[..7]
    } else {
        &default_days[..7]
    };

    let col_width = (width - 6.0 * 6.0) / 7.0;

    let mut cols: Vec<Box<dyn Widget>> = Vec::with_capacity(7);

    for (i, day) in days_to_show.iter().enumerate() {
        cols.push(Box::new(forecast_day_col(theme, day, col_width, i == 0)));
    }

    Row::new(cols)
        .width(width)
        .gap(6.0)
        .align(Center)
}

fn forecast_day_col(theme: &Theme, day: &ForecastDay, width: f32, is_today: bool) -> Column {
    let day_text = Text::new(&day.day_name)
        .size(11.0)
        .font(fonts::BODY)
        .weight(if is_today { Weight::Bold } else { Weight::Medium })
        .color(if is_today { theme.accent } else { theme.text });

    let icon_glyph = weather_code_icon(day.code);
    let icon_color = if is_today { theme.accent } else { theme.secondary_text };
    let icon_text = Text::new(icon_glyph)
        .size(17.0)
        .font(fonts::NERD)
        .tight()
        .color(icon_color);

    let icon_box = if is_today {
        Rectangle::new()
            .width(32.0)
            .height(26.0)
            .radius(8.0)
            .fill(theme.selected_surface)
            .align_child(Center, Center)
            .child(icon_text)
    } else {
        Rectangle::new()
            .width(32.0)
            .height(26.0)
            .align_child(Center, Center)
            .child(icon_text)
    };

    let high_text = Text::new(format!("{:.0}°", day.high))
        .size(10.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let low_text = Text::new(format!("{:.0}°", day.low))
        .size(10.0)
        .font(fonts::BODY)
        .color(theme.muted_text);

    let temps = Row::new(children![high_text, low_text]).gap(4.0).align(Center);

    Column::new(children![day_text, icon_box, temps])
        .width(width)
        .gap(3.0)
        .align(Center)
}

fn get_default_forecast() -> Vec<ForecastDay> {
    let clock = Clock::read();
    let today = clock.today();
    let (year, month, day) = today;
    let today_days = clock::days_since_1970(year, month, day);

    let sample_temps: [(f32, f32, u32); 7] = [
        (75.0, 52.0, 1),
        (79.0, 49.0, 3),
        (81.0, 51.0, 1),
        (85.0, 65.0, 2),
        (85.0, 68.0, 3),
        (88.0, 68.0, 3),
        (83.0, 68.0, 61),
    ];

    let mut out = Vec::new();
    for i in 0..7 {
        let date_days = today_days + i as i64;
        let day_name = match date_days.rem_euclid(7) {
            0 => "Thu",
            1 => "Fri",
            2 => "Sat",
            3 => "Sun",
            4 => "Mon",
            5 => "Tue",
            _ => "Wed",
        };
        let (h, l, c) = sample_temps[i];
        out.push(ForecastDay {
            day_name: day_name.to_string(),
            code: c,
            high: h,
            low: l,
            is_today: i == 0,
        });
    }
    out
}

fn bottom_section(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Row {
    let gap = 12.0;
    let left_width = ((width - gap) * 0.44).round();
    let right_width = width - gap - left_width;

    let left_col = left_column(overlay, theme, left_width, height);
    let right_card = calendar_card(overlay, theme, right_width, height);

    Row::new(children![left_col, right_card])
        .width(width)
        .height(height)
        .gap(gap)
        .align(Center)
}

fn left_column(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Column {
    let gap = 10.0;
    let half_height = ((height - gap) / 2.0).round();
    let reminders_height = height - half_height - gap;

    let notifications = notifications_card(overlay, theme, width, half_height);
    let reminders = reminders_card(overlay, theme, width, reminders_height);

    Column::new(children![notifications, reminders])
        .width(width)
        .height(height)
        .gap(gap)
}

fn notifications_card(_overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Rectangle {
    let notifications = Notifications::read();
    let count = notifications.list().len();

    let title_text = Text::new("Notifications")
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let count_str = if count == 0 {
        String::from("No new")
    } else if count == 1 {
        String::from("1 new")
    } else {
        format!("{count} new")
    };

    let count_text = Text::new(count_str)
        .size(11.0)
        .font(fonts::BODY)
        .color(if count > 0 { theme.accent } else { theme.muted_text });

    let header_title = Row::new(children![
        Text::new("󰂚 ")
            .size(13.5)
            .font(fonts::NERD)
            .color(if count > 0 { theme.accent } else { theme.secondary_text }),
        title_text,
        Text::new(" • ")
            .size(11.0)
            .font(fonts::BODY)
            .color(theme.border),
        count_text,
    ])
    .align(Center);

    let mut header_children: Vec<Box<dyn Widget>> = vec![Box::new(header_title)];

    if count > 0 {
        let clear_btn = Rectangle::new()
            .width(56.0)
            .height(20.0)
            .radius(4.0)
            .cursor(Pointer)
            .on_click(|_| Notifications::clear())
            .align_child(Center, Center)
            .child(
                Text::new("Clear all")
                    .size(11.0)
                    .font(fonts::BODY)
                    .weight(Weight::Medium)
                    .color(theme.danger),
            );
        header_children.push(Box::new(clear_btn));
    }

    let header_row = Row::new(header_children)
        .width(width - 32.0)
        .justify(SpaceBetween)
        .align(Center);

    let inner_width = width - 32.0;

    let mut notif_rows: Vec<Box<dyn Widget>> = Vec::new();

    if count == 0 {
        let empty_icon = Text::new("󰂜")
            .size(20.0)
            .font(fonts::NERD)
            .color(theme.muted_text);
        let empty_label = Text::new("No notifications")
            .size(11.5)
            .font(fonts::BODY)
            .color(theme.muted_text);
        let empty_col = Column::new(children![empty_icon, empty_label])
            .gap(4.0)
            .align(Center);

        notif_rows.push(Box::new(
            Rectangle::new()
                .width(inner_width)
                .height((height - 52.0).max(40.0))
                .align_child(Center, Center)
                .child(empty_col),
        ));
    } else {
        for n in notifications.list().iter().rev() {
            notif_rows.push(Box::new(notification_item(theme, n, inner_width)));
        }
    }

    let scroll_height = (height - 48.0).max(36.0);
    let scroll_area = ScrollArea::new(
        "overview-notifs-scroll",
        Column::new(notif_rows).gap(6.0).width(inner_width),
    )
    .width(inner_width)
    .height(scroll_height);

    let col = Column::new(children![header_row, scroll_area])
        .gap(6.0)
        .width(inner_width);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(20.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 12.0,
            right: 16.0,
            bottom: 12.0,
            left: 16.0,
        })
        .child(col)
}

fn notification_item(theme: &Theme, notification: &Notification, width: f32) -> Rectangle {
    let app_name = notification.app_name();
    let summary = notification.summary();
    let body = notification.body();
    let id = notification.id();

    let icon_box = Rectangle::new()
        .width(26.0)
        .height(26.0)
        .radius(7.0)
        .fill(theme.selected_surface)
        .align_child(Center, Center)
        .child(
            Text::new("󰂚")
                .size(13.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.accent),
        );

    let display_title = if !summary.is_empty() {
        summary
    } else if !app_name.is_empty() {
        app_name
    } else {
        "Notification"
    };

    let title_w = Text::new(display_title)
        .size(12.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text)
        .elide();

    let sub_text = if !body.is_empty() {
        body
    } else if !summary.is_empty() && !app_name.is_empty() {
        app_name
    } else {
        ""
    };

    let sub_w = Text::new(sub_text)
        .size(10.5)
        .font(fonts::BODY)
        .color(theme.secondary_text)
        .elide();

    let text_width = (width - 26.0 - 10.0 - 24.0 - 20.0).max(40.0);

    let text_col = Column::new(children![
        Rectangle::new().width(text_width).height(16.0).align_child(Start, Center).child(title_w),
        Rectangle::new().width(text_width).height(14.0).align_child(Start, Center).child(sub_w),
    ]).gap(1.0);

    let left = Row::new(children![icon_box, text_col])
        .gap(8.0)
        .align(Center);

    let dismiss_btn = Rectangle::new()
        .width(20.0)
        .height(20.0)
        .radius(Full)
        .cursor(Pointer)
        .on_click(move |_| Notifications::dismiss(id))
        .align_child(Center, Center)
        .child(
            Text::new("\u{f00d}")
                .size(11.0)
                .font(fonts::NERD)
                .tight()
                .color(theme.muted_text),
        );

    let row = Row::new(children![left, dismiss_btn])
        .width(width - 18.0)
        .justify(SpaceBetween)
        .align(Center);

    let mut card = Rectangle::new()
        .width(width)
        .height(44.0)
        .radius(10.0)
        .fill(theme.hover_surface)
        .padding(Padding {
            top: 5.0,
            right: 8.0,
            bottom: 5.0,
            left: 10.0,
        })
        .align_child(Start, Center);

    if notification.has_default_action() {
        card = card.cursor(Pointer).on_click(move |_| Notifications::click(id));
    }

    card.child(row)
}

fn flectar_event_row(
    theme: &Theme,
    event: &crate::integrations::flectar::CalendarEvent,
    offset_secs: i64,
    width: f32,
) -> Rectangle {
    let time_str = if event.all_day {
        String::from("All day")
    } else {
        let local_secs = event.starts_at + offset_secs;
        let day_secs = local_secs.rem_euclid(86400);
        let h = day_secs / 3600;
        let m = (day_secs % 3600) / 60;
        let ampm = if h >= 12 { "PM" } else { "AM" };
        let h12 = match h % 12 {
            0 => 12,
            other => other,
        };
        format!("{h12}:{m:02} {ampm}")
    };

    let summary = Text::new(&event.summary)
        .size(12.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let mut subtitle_parts = vec![time_str];
    if let Some(loc) = &event.location {
        if !loc.trim().is_empty() {
            subtitle_parts.push(loc.trim().to_string());
        }
    }

    let sub_text = Text::new(subtitle_parts.join(" • "))
        .size(10.5)
        .font(fonts::BODY)
        .color(theme.muted_text);

    let icon = Text::new(if event.join_url.is_some() { "󰍫 " } else { "󰸗 " })
        .size(13.0)
        .font(fonts::NERD)
        .color(theme.accent);

    let text_col = Column::new(children![summary, sub_text]).gap(2.0);
    let content_row = Row::new(children![icon, text_col]).gap(8.0).align(Center);

    let join_url = event.join_url.clone();
    Rectangle::new()
        .width(width)
        .height(44.0)
        .radius(10.0)
        .fill(theme.hover_surface)
        .padding(Padding {
            top: 5.0,
            right: 10.0,
            bottom: 5.0,
            left: 10.0,
        })
        .cursor(Pointer)
        .align_child(Start, Center)
        .child(content_row)
        .on_click(move |_| {
            if let Some(url) = &join_url {
                if !url.trim().is_empty() {
                    amane::spawn(&format!("xdg-open '{url}'"));
                    return;
                }
            }
            amane::spawn("gtk-launch com.flectar.mail 2>/dev/null || flectar-mail");
        })
}

fn reminders_card(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Rectangle {
    let clock = Clock::read();
    let today = clock.today();
    let (sel_year, sel_month, sel_day) = overlay.selected_calendar_day.unwrap_or(today);
    let is_today = (sel_year, sel_month, sel_day) == today;

    let sel_days = clock::days_since_1970(sel_year, sel_month, sel_day);
    let weekday = match (sel_days + 4).rem_euclid(7) {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        _ => "Sat",
    };
    let month_name = MONTHS[(sel_month - 1) as usize];
    let title_str = if is_today {
        format!("Today, {month_name} {sel_day}")
    } else {
        format!("{weekday}, {month_name} {sel_day}")
    };

    let title_text = Text::new(title_str)
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let offset_secs = clock.offset();
    let flectar_events = crate::integrations::flectar::events_for_day(offset_secs, sel_year, sel_month, sel_day);
    let reminders = RemindersData::read();

    let total_count = flectar_events.len() + if is_today { reminders.items.len() } else { 0 };
    let count_str = if total_count == 1 {
        "1 item".to_string()
    } else {
        format!("{total_count} items")
    };

    let count_text = Text::new(count_str)
        .size(11.0)
        .font(fonts::BODY)
        .color(theme.muted_text);

    let header_title = Row::new(children![
        Text::new("󰸗 ")
            .size(13.5)
            .font(fonts::NERD)
            .color(theme.accent),
        title_text,
        Text::new(" • ")
            .size(11.0)
            .font(fonts::BODY)
            .color(theme.border),
        count_text,
    ])
    .align(Center);

    let open_row = Row::new(children![
        Text::new("󰅒 ")
            .size(11.5)
            .font(fonts::NERD)
            .color(theme.accent),
        Text::new("Mail")
            .size(11.0)
            .font(fonts::BODY)
            .weight(Weight::Medium)
            .color(theme.accent),
    ])
    .gap(4.0)
    .align(Center);

    let open_flectar_btn = Rectangle::new()
        .width(52.0)
        .height(20.0)
        .cursor(Pointer)
        .padding(Padding {
            top: 2.0,
            bottom: 2.0,
            left: 6.0,
            right: 6.0,
        })
        .radius(6.0)
        .fill(theme.hover_surface)
        .align_child(Center, Center)
        .child(open_row)
        .on_click(|_| {
            amane::spawn("gtk-launch com.flectar.mail 2>/dev/null || flectar-mail");
        });

    let inner_width = width - 32.0;

    let header_row = Row::new(children![
        header_title,
        open_flectar_btn,
    ])
    .width(inner_width)
    .justify(SpaceBetween)
    .align(Center);

    let mut reminder_rows: Vec<Box<dyn Widget>> = Vec::new();

    for event in &flectar_events {
        reminder_rows.push(Box::new(flectar_event_row(theme, event, offset_secs, inner_width)));
    }

    if is_today {
        for item in &reminders.items {
            reminder_rows.push(Box::new(reminder_row(theme, item, inner_width)));
        }
    }

    if reminder_rows.is_empty() {
        let empty_msg = if is_today {
            "No events or reminders today"
        } else {
            "No events scheduled for this day"
        };
        let empty_text = Text::new(empty_msg)
            .size(11.5)
            .font(fonts::BODY)
            .color(theme.muted_text);
        reminder_rows.push(Box::new(
            Rectangle::new()
                .width(inner_width)
                .height(36.0)
                .align_child(Center, Center)
                .child(empty_text),
        ));
    }

    let scroll_height = (height - 90.0).max(36.0);
    let scroll_area = ScrollArea::new(
        "reminders-scroll",
        Column::new(reminder_rows).gap(6.0).width(inner_width),
    )
    .width(inner_width)
    .height(scroll_height);

    let input_bar = reminder_input_bar(theme, inner_width);

    let col = Column::new(children![header_row, scroll_area, input_bar])
        .gap(6.0)
        .width(inner_width);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(20.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 12.0,
            right: 16.0,
            bottom: 10.0,
            left: 16.0,
        })
        .child(col)
}

fn reminder_row(theme: &Theme, item: &ReminderItem, width: f32) -> Rectangle {
    let circle_icon = Text::new("󰄰")
        .size(13.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.secondary_text);

    let tag_w = item.tag.chars().count() as f32 * 7.0 + 12.0;
    let tag_badge = Rectangle::new()
        .width(tag_w.max(30.0))
        .height(18.0)
        .radius(5.0)
        .fill(theme.selected_surface)
        .padding(Padding {
            top: 2.0,
            bottom: 2.0,
            left: 4.0,
            right: 4.0,
        })
        .align_child(Center, Center)
        .child(
            Text::new(&item.tag)
                .size(10.5)
                .font(fonts::BODY)
                .weight(Weight::Medium)
                .color(theme.secondary_text),
        );

    let desc_width = (width - 16.0 - 55.0 - 24.0 - 16.0).max(40.0);
    let desc_text = Text::new(&item.text)
        .size(12.0)
        .font(fonts::BODY)
        .color(theme.text)
        .elide();

    let desc_box = Rectangle::new()
        .width(desc_width)
        .height(20.0)
        .align_child(Start, Center)
        .child(desc_text);

    let left = Row::new(children![circle_icon, tag_badge, desc_box])
        .gap(6.0)
        .align(Center);

    let id = item.id;
    let delete_icon = Text::new("\u{f00d}")
        .size(11.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.muted_text);

    let delete_btn = Rectangle::new()
        .width(18.0)
        .height(18.0)
        .radius(4.0)
        .cursor(Pointer)
        .on_click(move |_| delete_reminder(id))
        .align_child(Center, Center)
        .child(delete_icon);

    let row = Row::new(children![left, delete_btn])
        .width(width)
        .justify(SpaceBetween)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(26.0)
        .align_child(Start, Center)
        .child(row)
}

fn reminder_input_bar(theme: &Theme, width: f32) -> Rectangle {
    let bullet = Text::new("●")
        .size(9.0)
        .font(fonts::BODY)
        .color(theme.accent);

    let input_w = (width - 24.0 - 58.0 - 12.0).max(40.0);
    let input = TextInput::new("new_reminder_input")
        .placeholder("New reminder: 3:30pm Call mom")
        .size(11.5)
        .color(theme.text)
        .width(input_w)
        .on_change(|val| {
            RemindersData::write().input_text = val;
        })
        .on_submit(|val| {
            add_reminder(&val);
        });

    let add_btn = Rectangle::new()
        .width(52.0)
        .height(24.0)
        .radius(12.0)
        .fill(theme.selected_surface)
        .cursor(Pointer)
        .on_click(|_| {
            let txt = RemindersData::read().input_text.clone();
            add_reminder(&txt);
        })
        .align_child(Center, Center)
        .child(
            Text::new("Add")
                .size(11.0)
                .font(fonts::BODY)
                .weight(Weight::SemiBold)
                .color(theme.text),
        );

    let row = Row::new(children![bullet, input, add_btn])
        .width(width - 16.0)
        .gap(6.0)
        .align(Center);

    Rectangle::new()
        .width(width)
        .height(30.0)
        .radius(8.0)
        .fill(theme.selected_surface)
        .padding(Padding {
            top: 3.0,
            right: 8.0,
            bottom: 3.0,
            left: 8.0,
        })
        .align_child(Center, Center)
        .child(row)
}

// Right card: Interactive month calendar grid with navigation
fn calendar_card(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = width - PADDING * 2.0;
    let inner_height = height - PADDING * 2.0;

    let chosen = overlay.calendar_month as f32;
    let shown = motion::follow("calendar-month", chosen, DEFAULT_SPATIAL);
    let today = Clock::read().today();

    let mut pages: Vec<Box<dyn Widget>> = Vec::new();
    let nearest = shown.floor() as i64;

    for offset in [nearest, nearest + 1] {
        let distance = offset as f32 - shown;
        if distance.abs() >= 1.0 {
            continue;
        }

        let page = month(overlay, theme, today, offset, inner_width, inner_height)
            .translate(distance * inner_width, 0.0);

        pages.push(Box::new(page));
    }

    let mut viewport = Rectangle::new()
        .width(inner_width)
        .height(inner_height)
        .child(Stack::new(pages));

    if shown != chosen {
        viewport = viewport.clip();
    }

    let navigation = Row::new(children![
        nav_button(overlay, theme, "prev", PREVIOUS_ICON, -1),
        today_button(theme),
        nav_button(overlay, theme, "next", NEXT_ICON, 1),
    ])
    .gap(4.0)
    .align(Center);

    let nav_bar = Rectangle::new()
        .width(inner_width)
        .height(NAVIGATION_SIZE)
        .align_child(End, Start)
        .child(navigation);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(20.0)
        .fill(theme.surface)
        .padding(Padding {
            top: PADDING,
            right: PADDING,
            bottom: PADDING,
            left: PADDING,
        })
        .child(Stack::new(children![viewport, nav_bar]))
}

fn today_button(theme: &Theme) -> Rectangle {
    Rectangle::new()
        .width(54.0)
        .height(NAVIGATION_SIZE)
        .radius(7.0)
        .fill(theme.selected_surface)
        .padding(Padding {
            top: 2.0,
            bottom: 2.0,
            left: 10.0,
            right: 10.0,
        })
        .cursor(Pointer)
        .on_click(|_| {
            let mut o = Overlay::write();
            o.calendar_month = 0;
            o.selected_calendar_day = None;
        })
        .align_child(Center, Center)
        .child(
            Text::new("Today")
                .size(11.0)
                .font(fonts::BODY)
                .weight(Weight::SemiBold)
                .color(theme.text),
        )
}

fn month(
    overlay: &Overlay,
    theme: &Theme,
    today: (i64, i64, i64),
    offset: i64,
    width: f32,
    height: f32,
) -> Rectangle {
    let (today_year, today_month, _) = today;
    let index = today_year * 12 + (today_month - 1) + offset;
    let year = index.div_euclid(12);
    let month = index.rem_euclid(12) + 1;

    let title_text = Text::new(format!("{} {year}", MONTHS[(month - 1) as usize]))
        .size(16.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let mut title_bar = Rectangle::new()
        .width(width - 120.0)
        .height(28.0)
        .align_child(Start, Center)
        .child(title_text);

    if offset != 0 {
        title_bar = title_bar
            .cursor(Pointer)
            .on_click(|_| {
            let mut o = Overlay::write();
            o.calendar_month = 0;
            o.selected_calendar_day = None;
        });
    }

    let weekdays_h = 18.0;
    let title_h = 28.0;
    let gap = 6.0;
    let grid_height = height - title_h - weekdays_h - gap * 2.0;

    let column = Column::new(children![
        title_bar,
        weekdays(theme, width),
        grid(overlay, theme, &days(today, year, month), offset, width, grid_height),
    ])
    .gap(gap);

    Rectangle::new().width(width).height(height).child(column)
}

fn weekdays(theme: &Theme, width: f32) -> Row {
    let cell_width = width / 7.0;
    let mut names: Vec<Box<dyn Widget>> = Vec::with_capacity(7);

    for name in WEEKDAYS {
        let text = Text::new(name)
            .size(11.0)
            .font(fonts::BODY)
            .weight(Weight::Medium)
            .color(theme.muted_text);

        let cell = Rectangle::new()
            .width(cell_width)
            .height(18.0)
            .align_child(Center, Center)
            .child(text);

        names.push(Box::new(cell));
    }

    Row::new(names)
}

fn grid(
    overlay: &Overlay,
    theme: &Theme,
    days: &[Day],
    offset: i64,
    width: f32,
    height: f32,
) -> Column {
    let cell_gap = 4.0;
    let cell_width = (width - cell_gap * 6.0) / 7.0;
    let cell_height = (height - cell_gap * (ROWS - 1) as f32) / ROWS as f32;

    let mut rows: Vec<Box<dyn Widget>> = Vec::with_capacity(ROWS);

    for week in 0..ROWS {
        let mut cells: Vec<Box<dyn Widget>> = Vec::with_capacity(7);

        for weekday in 0..7 {
            let index = week * 7 + weekday;
            let hover_name = format!("cal:{offset}:{index}");

            let cell = cell(overlay, theme, &days[index], hover_name, cell_width, cell_height);
            cells.push(Box::new(cell));
        }

        rows.push(Box::new(Row::new(cells).gap(cell_gap)));
    }

    Column::new(rows).gap(cell_gap)
}

fn cell(
    overlay: &Overlay,
    theme: &Theme,
    day: &Day,
    hover_name: String,
    width: f32,
    height: f32,
) -> Rectangle {
    let is_hovered = overlay.hovered.as_deref() == Some(&hover_name);
    let is_selected = match overlay.selected_calendar_day {
        Some((y, m, d)) => y == day.year && m == day.month && d == day.number,
        None => false,
    };

    let (dy, dm, dn) = (day.year, day.month, day.number);

    if day.today {
        let circle_size = width.min(height).min(38.0);

        let num = Text::new(day.number.to_string())
            .size(12.5)
            .font(fonts::BODY)
            .weight(Weight::Bold)
            .color(theme.on_accent);

        let dot = Text::new("●")
            .size(5.0)
            .font(fonts::BODY)
            .color(theme.on_accent);

        let inner_col = Column::new(children![num, dot]).gap(1.0).align(Center);

        let circle = Rectangle::new()
            .width(circle_size)
            .height(circle_size)
            .radius(circle_size / 2.0)
            .fill(theme.accent)
            .align_child(Center, Center)
            .child(inner_col);

        return Rectangle::new()
            .width(width)
            .height(height)
            .cursor(Pointer)
            .align_child(Center, Center)
            .child(circle)
            .on_click(move |_| {
                Overlay::write().selected_calendar_day = Some((dy, dm, dn));
            });
    }

    let fill = if is_selected {
        theme.selected_surface
    } else if day.this_month && is_hovered {
        theme.hover_surface
    } else {
        Color::TRANSPARENT
    };

    let color = if is_selected {
        theme.accent
    } else if day.this_month {
        theme.text
    } else {
        theme::with_opacity(theme.secondary_text, 0.4)
    };

    let weight = if is_selected || day.this_month {
        Weight::Medium
    } else {
        Weight::Light
    };

    let number = Text::new(day.number.to_string())
        .size(12.0)
        .font(fonts::BODY)
        .weight(weight)
        .color(color);

    let hover_key = hover_name.clone();

    let mut rect = Rectangle::new()
        .width(width)
        .height(height)
        .radius(8.0)
        .fill(fill)
        .on_hover(move |inside| {
            let needs_change = {
                let o = Overlay::read();
                if inside {
                    o.hovered.as_ref() != Some(&hover_key)
                } else {
                    o.hovered.as_ref() == Some(&hover_key)
                }
            };
            if needs_change {
                let mut o = Overlay::write();
                if inside {
                    o.hovered = Some(hover_key.clone());
                } else if o.hovered.as_ref() == Some(&hover_key) {
                    o.hovered = None;
                }
            }
        });

    if day.this_month {
        rect = rect.cursor(Pointer).on_click(move |_| {
            Overlay::write().selected_calendar_day = Some((dy, dm, dn));
        });
    }

    if day.has_event && day.this_month {
        let dot = Text::new("●")
            .size(5.0)
            .font(fonts::BODY)
            .color(theme.accent);
        rect.align_child(Center, Center)
            .child(Column::new(children![number, dot]).gap(1.0).align(Center))
    } else {
        rect.align_child(Center, Center).child(number)
    }
}

struct Day {
    number: i64,
    this_month: bool,
    today: bool,
    has_event: bool,
    year: i64,
    month: i64,
}

fn days(today: (i64, i64, i64), year: i64, month: i64) -> Vec<Day> {
    let first = clock::days_since_1970(year, month, 1);
    let weekday = (first + 4).rem_euclid(7);
    let start = first - weekday;

    let (today_year, today_month, today_day) = today;
    let today_days = clock::days_since_1970(today_year, today_month, today_day);
    let offset_secs = Clock::read().offset();

    let mut days = Vec::with_capacity(ROWS * 7);

    for index in 0..(ROWS * 7) as i64 {
        let date = start + index;
        let (date_year, date_month, date_day) = clock::civil_date(date);
        let has_event = crate::integrations::flectar::has_event_on(offset_secs, date_year, date_month, date_day);

        days.push(Day {
            number: date_day,
            this_month: date_month == month,
            today: date == today_days,
            has_event,
            year: date_year,
            month: date_month,
        });
    }

    days
}

fn nav_button(
    overlay: &Overlay,
    theme: &Theme,
    name: &str,
    icon: &str,
    step: i64,
) -> Rectangle {
    let hover_name = format!("cal_nav:{name}");
    let is_hovered = overlay.hovered.as_deref() == Some(&hover_name);

    let fill = if is_hovered {
        theme.border
    } else {
        theme.selected_surface
    };

    let glyph = Text::new(icon)
        .size(11.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.text);

    let hover_key = hover_name.clone();

    Rectangle::new()
        .width(NAVIGATION_SIZE)
        .height(NAVIGATION_SIZE)
        .radius(7.0)
        .fill(fill)
        .cursor(Pointer)
        .on_click(move |_| Overlay::write().calendar_month += step)
        .on_hover(move |inside| {
            let needs_change = {
                let o = Overlay::read();
                if inside {
                    o.hovered.as_ref() != Some(&hover_key)
                } else {
                    o.hovered.as_ref() == Some(&hover_key)
                }
            };
            if needs_change {
                let mut o = Overlay::write();
                if inside {
                    o.hovered = Some(hover_key.clone());
                } else if o.hovered.as_ref() == Some(&hover_key) {
                    o.hovered = None;
                }
            }
        })
        .align_child(Center, Center)
        .child(glyph)
}

pub fn ipc(arguments: &[String]) -> String {
    let monitor_arg = arguments.iter().find(|arg| !matches!(arg.as_str(), "toggle" | "show" | "hide")).map(String::as_str);
    Overlay::toggle_calendar(monitor_arg);
    String::from("ok")
}
