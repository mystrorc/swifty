use std::cell::RefCell;

use amane::{
    Apps, Center, Color, Column, Image, Key, Padding, Pointer, Rectangle, Row,
    Scroll, Service, Start, Text, TextInput, Weight, Widget, children,
};

use super::{Overlay, PanelView, Region};
use crate::fonts;
use crate::liquid::Blob;
use crate::settings::Settings;
use crate::theme::Theme;

const INPUT: &str = "app_menu_search";

const WIDTH: f32 = 760.0;
const HEIGHT: f32 = 560.0;
const RADIUS: f32 = 28.0;

const TOP_OFFSET: f32 = 8.0;
const SHADOW_MARGIN: f32 = 20.0;

const PADDING: Padding = Padding {
    top: 16.0,
    right: 18.0,
    bottom: 16.0,
    left: 18.0,
};

const SEARCH_HEIGHT: f32 = 42.0;
const CATEGORIES_HEIGHT: f32 = 30.0;
const FOOTER_HEIGHT: f32 = 26.0;
const GAP: f32 = 12.0;

const TILE_HEIGHT: f32 = 68.0;
const TILE_GAP: f32 = 10.0;
const COLUMNS: usize = 4;
const ICON_SIZE: f32 = 36.0;

const CATEGORIES: [(&str, &str); 7] = [
    ("All", "\u{f009}"),          // th-large / grid
    ("Development", "\u{f121}"),  // code
    ("Internet", "\u{f0ac}"),     // globe
    ("Multimedia", "\u{f008}"),   // film / play
    ("Office", "\u{f15c}"),       // file-text
    ("System", "\u{f013}"),       // cog
    ("Utilities", "\u{f0ad}"),    // wrench
];

struct AppItem {
    index: usize,
    name: String,
    category: &'static str,
    description: Option<String>,
}

#[derive(Clone)]
struct CachedApp {
    index: usize,
    name: String,
    category: &'static str,
    description: Option<String>,
    search_haystack: String,
}

thread_local! {
    static CACHED_MENU_APPS: RefCell<Option<(usize, Vec<CachedApp>)>> = const { RefCell::new(None) };
}

fn get_cached_menu_apps() -> Vec<CachedApp> {
    let apps = Apps::read();
    let list = apps.list();
    let count = list.len();

    CACHED_MENU_APPS.with_borrow_mut(|cache| {
        if let Some((cached_count, items)) = cache.as_ref() {
            if *cached_count == count {
                return items.clone();
            }
        }

        let mut items = Vec::with_capacity(count);
        for (index, app) in list.iter().enumerate() {
            let name = app.name();
            let desc = app.description();
            let exec = app.exec();
            let category = categorize(name, desc, exec);

            let desc_str = desc.map(String::from);
            let search_haystack = format!("{} {} {}", name, desc.unwrap_or(""), category).to_lowercase();

            items.push(CachedApp {
                index,
                name: String::from(name),
                category,
                description: desc_str,
                search_haystack,
            });
        }

        *cache = Some((count, items.clone()));
        items
    })
}

// none while fully hidden
pub fn view(overlay: &Overlay, theme: &Theme, screen: Region) -> Option<PanelView> {
    let progress = overlay.app_menu.progress.value();

    if !overlay.app_menu.shown && progress <= 0.001 {
        return None;
    }

    let target_width = WIDTH.min(screen.width - 24.0);
    let target_height = HEIGHT.min(screen.height - 24.0);
    let settings = Settings::read();
    let transition_style = settings.text("panel_transition");

    let (x, y, current_width, current_height, radius, content_opacity) = match transition_style {
        "slide" => {
            let hidden = -(target_height + 32.0);
            let y = TOP_OFFSET + (hidden - TOP_OFFSET) * (1.0 - progress);
            (12.0, y, target_width, target_height, RADIUS, 1.0)
        }
        "fade" => {
            let scale = 0.92 + 0.08 * progress;
            let current_w = target_width * scale;
            let current_h = target_height * scale;
            let x = 12.0;
            let y = TOP_OFFSET + (target_height - current_h) / 2.0;
            (x, y, current_w, current_h, RADIUS, progress.clamp(0.0, 1.0))
        }
        _ => {
            // "drip" - Liquid Drip / Stretch from distro pill
            let start_width = 34.0f32.min(target_width);
            let start_height = 24.0f32;

            let h_t = progress.powf(0.8);
            let current_h = (start_height + (target_height - start_height) * h_t).min(target_height);

            let w_t = progress.powf(1.25);
            let current_w = (start_width + (target_width - start_width) * w_t).min(target_width);

            let y = TOP_OFFSET;
            let x = 12.0;

            let capsule_r = (current_h / 2.0).min(current_w / 2.0);
            let radius = capsule_r * (1.0 - progress) + RADIUS * progress;

            let opacity = progress.powf(1.8);
            (x, y, current_w, current_h, radius, opacity)
        }
    };

    let content_box = content(overlay, theme, target_width, target_height);
    let wrapped_content = if progress < 0.999 {
        Rectangle::new()
            .width(target_width)
            .height(target_height)
            .opacity(content_opacity)
            .child(content_box)
    } else {
        content_box
    };

    let blob = Blob::new(x, y, current_width, current_height)
        .radius(radius)
        .child(wrapped_content);

    let input = Region {
        x,
        y,
        width: current_width,
        height: current_height,
    };

    let reach = Region {
        x: (12.0 - SHADOW_MARGIN).max(0.0),
        y: 0.0,
        width: target_width + SHADOW_MARGIN * 2.0,
        height: TOP_OFFSET + target_height + SHADOW_MARGIN,
    };

    Some(PanelView { blob, input, reach })
}

fn content(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Rectangle {
    let inner_width = width - PADDING.left - PADDING.right;

    let search_bar = search_bar(overlay, theme, inner_width);
    let category_bar = category_bar(overlay, theme, inner_width);

    let apps = filtered_apps(overlay);
    let app_count = apps.len();

    let grid_height = height
        - PADDING.top
        - PADDING.bottom
        - SEARCH_HEIGHT
        - CATEGORIES_HEIGHT
        - FOOTER_HEIGHT
        - GAP * 3.0;

    let tiles_grid = grid(overlay, theme, &apps, inner_width, grid_height);

    let footer = footer(theme, app_count, inner_width);

    let column = Column::new(children![
        search_bar,
        category_bar,
        tiles_grid,
        footer,
    ])
    .gap(GAP);

    Rectangle::new()
        .width(width)
        .height(height)
        .padding(PADDING)
        .child(column)
}

fn search_bar(_overlay: &Overlay, theme: &Theme, width: f32) -> Rectangle {
    let search_icon = Text::new("\u{f002}") // search icon
        .size(14.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.secondary_text);

    let input = TextInput::new(INPUT)
        .size(14.0)
        .color(theme.text)
        .placeholder("Search applications...")
        .focused()
        .on_change(|text| {
            let mut o = Overlay::write();
            o.app_menu_query = text;
            o.app_menu_scroll = 0.0;
        })
        .on_submit(|_| launch_first_result());

    let field_width = width - 40.0;

    let row = Row::new(children![
        search_icon,
        Rectangle::new().width(field_width).height(SEARCH_HEIGHT).align_child(Start, Center).child(input),
    ])
    .gap(12.0)
    .align(Center);

    Rectangle::new()
        .width(width)
        .height(SEARCH_HEIGHT)
        .radius(14.0)
        .fill(theme.surface)
        .padding(Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 14.0,
        })
        .align_child(Start, Center)
        .child(row)
}

fn category_bar(overlay: &Overlay, theme: &Theme, width: f32) -> Row {
    let mut chips: Vec<Box<dyn Widget>> = Vec::new();
    let gap = 6.0;
    let count = CATEGORIES.len() as f32;
    let chip_width = ((width - gap * (count - 1.0)) / count).floor();

    for (idx, (name, icon)) in CATEGORIES.iter().enumerate() {
        let active = overlay.app_menu_category == idx;

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

        let icon_color = if active {
            theme.on_accent
        } else {
            theme.accent
        };

        let icon_w = Text::new(*icon)
            .size(12.0)
            .font(fonts::NERD)
            .tight()
            .color(icon_color);

        let label_w = Text::new(*name)
            .size(11.0)
            .font(fonts::BODY)
            .weight(if active { Weight::SemiBold } else { Weight::Medium })
            .color(text_color);

        let chip_content = Row::new(children![icon_w, label_w])
            .gap(5.0)
            .align(Center);

        let chip = Rectangle::new()
            .width(chip_width)
            .height(CATEGORIES_HEIGHT)
            .radius(10.0)
            .fill(fill)
            .cursor(Pointer)
            .on_click(move |_| {
                let mut o = Overlay::write();
                o.app_menu_category = idx;
                o.app_menu_scroll = 0.0;
            })
            .align_child(Center, Center)
            .child(chip_content);

        chips.push(Box::new(chip));
    }

    Row::new(chips)
        .width(width)
        .gap(gap)
        .align(Center)
}

fn grid(
    overlay: &Overlay,
    theme: &Theme,
    apps: &[AppItem],
    width: f32,
    height: f32,
) -> Rectangle {
    let area = Rectangle::new()
        .width(width)
        .height(height)
        .clip()
        .on_scroll(scrolled);

    if apps.is_empty() {
        let empty = Text::new("No matching applications found")
            .size(14.0)
            .font(fonts::BODY)
            .color(theme.secondary_text);

        return area.align_child(Center, Center).child(empty);
    }

    let col_width = (width - TILE_GAP * (COLUMNS - 1) as f32) / COLUMNS as f32;
    let row_height = TILE_HEIGHT;
    let row_step = row_height + TILE_GAP;

    let total_rows = (apps.len() + COLUMNS - 1) / COLUMNS;
    let visible_rows_count = (height / row_step).ceil() as usize + 1;

    let scroll_row = overlay.app_menu_scroll.max(0.0);
    let start_row = (scroll_row as usize).min(total_rows.saturating_sub(1));
    let end_row = (start_row + visible_rows_count).min(total_rows);

    let offset_y = (scroll_row - start_row as f32) * row_step;

    let mut row_widgets: Vec<Box<dyn Widget>> = Vec::new();

    for row_idx in start_row..end_row {
        let mut col_widgets: Vec<Box<dyn Widget>> = Vec::new();

        for col_idx in 0..COLUMNS {
            let app_idx = row_idx * COLUMNS + col_idx;

            if app_idx < apps.len() {
                col_widgets.push(Box::new(tile(overlay, theme, &apps[app_idx], col_width, row_height)));
            } else {
                col_widgets.push(Box::new(Rectangle::new().width(col_width).height(row_height)));
            }
        }

        let row = Row::new(col_widgets).gap(TILE_GAP);
        row_widgets.push(Box::new(row));
    }

    let content_height = (end_row - start_row) as f32 * row_step;

    let content = Rectangle::new()
        .width(width)
        .height(content_height)
        .translate(0.0, -offset_y)
        .child(Column::new(row_widgets).gap(TILE_GAP));

    area.child(content)
}

fn tile(
    overlay: &Overlay,
    theme: &Theme,
    item: &AppItem,
    width: f32,
    height: f32,
) -> Rectangle {
    let hovered = overlay.app_menu_hovered == Some(item.index);

    let fill = if hovered {
        theme.selected_surface
    } else {
        theme.surface
    };

    let border_color = if hovered {
        theme.accent
    } else {
        Color::TRANSPARENT
    };

    let icon_w = app_icon(item.index, &item.name, theme);

    let name_w = Text::new(&item.name)
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text)
        .elide();

    let cat_text = item.description.as_deref().unwrap_or(item.category);
    let cat_w = Text::new(cat_text)
        .size(10.0)
        .font(fonts::BODY)
        .color(theme.secondary_text)
        .elide();

    let text_column = Column::new(children![name_w, cat_w])
        .gap(2.0)
        .width(width - ICON_SIZE - 24.0);

    let row = Row::new(children![icon_w, text_column])
        .gap(10.0)
        .align(Center);

    let app_index = item.index;

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(14.0)
        .fill(fill)
        .border(1.0, border_color)
        .padding(Padding {
            top: 0.0,
            right: 8.0,
            bottom: 0.0,
            left: 10.0,
        })
        .cursor(Pointer)
        .on_hover(move |inside| {
            let mut o = Overlay::write();
            if inside {
                o.app_menu_hovered = Some(app_index);
            } else if o.app_menu_hovered == Some(app_index) {
                o.app_menu_hovered = None;
            }
        })
        .on_click(move |_| {
            let apps = Apps::read();
            if let Some(app) = apps.list().get(app_index) {
                app.launch();
            }
            Overlay::toggle_app_menu(None);
        })
        .align_child(Start, Center)
        .child(row)
}

fn app_icon(index: usize, name: &str, theme: &Theme) -> Rectangle {
    let slot = Rectangle::new()
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .radius(8.0)
        .align_child(Center, Center);

    let apps = Apps::read();
    if let Some(app) = apps.list().get(index) {
        if let Some(path) = app.icon_path() {
            let readable = matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("png" | "jpg" | "jpeg" | "svg")
            );

            if readable {
                let pixels = (ICON_SIZE * 2.0) as u32;
                return slot.fill(Image::contain(path).thumbnail(pixels, pixels));
            }
        }
    }

    // Fallback: stylish tile badge with first letter of app name
    let letter = name.chars().next().unwrap_or('A').to_uppercase().to_string();
    let badge_text = Text::new(letter)
        .size(16.0)
        .font(fonts::BODY)
        .weight(Weight::Bold)
        .color(theme.accent);

    slot.fill(theme.selected_surface).child(badge_text)
}

fn footer(theme: &Theme, count: usize, width: f32) -> Row {
    let count_text = format!("{count} Applications");
    let count_label = Text::new(count_text)
        .size(11.0)
        .font(fonts::BODY)
        .color(theme.muted_text);

    let tip = Text::new("Press Esc to close  \u{2022}  Click to launch")
        .size(11.0)
        .font(fonts::BODY)
        .color(theme.muted_text);

    Row::new(children![count_label, tip])
        .width(width)
        .justify(amane::SpaceBetween)
        .align(Center)
}

fn scrolled(scroll: Scroll) {
    let mut overlay = Overlay::write();
    let change = if scroll.y < 0.0 { -1.0 } else { 1.0 };

    let total_apps = filtered_apps(&overlay).len();
    let total_rows = (total_apps + COLUMNS - 1) / COLUMNS;
    let max_scroll = (total_rows as f32 - 4.0).max(0.0);

    overlay.app_menu_scroll = (overlay.app_menu_scroll + change).clamp(0.0, max_scroll);
}

fn filtered_apps(overlay: &Overlay) -> Vec<AppItem> {
    let cached = get_cached_menu_apps();
    let search = overlay.app_menu_query.trim().to_lowercase();
    let cat_filter = overlay.app_menu_category;

    let target_cat = if cat_filter < CATEGORIES.len() {
        CATEGORIES[cat_filter].0
    } else {
        "All"
    };

    let mut result = Vec::new();

    for item in cached {
        if target_cat != "All" && item.category != target_cat {
            continue;
        }

        if !search.is_empty() && !item.search_haystack.contains(&search) {
            continue;
        }

        result.push(AppItem {
            index: item.index,
            name: item.name,
            category: item.category,
            description: item.description,
        });
    }

    result
}

fn categorize(name: &str, desc: Option<&str>, exec: &str) -> &'static str {
    let text = format!("{} {} {}", name, desc.unwrap_or(""), exec).to_lowercase();

    if text.contains("code") || text.contains("develop") || text.contains("terminal")
        || text.contains("vim") || text.contains("git") || text.contains("zed")
        || text.contains("alacritty") || text.contains("kitty") || text.contains("foot")
        || text.contains("rust") || text.contains("python") || text.contains("compiler")
        || text.contains("debugger") || text.contains("builder") || text.contains("neovim") {
        return "Development";
    }

    if text.contains("browser") || text.contains("firefox") || text.contains("chrome")
        || text.contains("chromium") || text.contains("brave") || text.contains("web")
        || text.contains("discord") || text.contains("vesktop") || text.contains("slack")
        || text.contains("telegram") || text.contains("mail") || text.contains("thunderbird")
        || text.contains("network") || text.contains("torrent") || text.contains("ftp") {
        return "Internet";
    }

    if text.contains("audio") || text.contains("video") || text.contains("music")
        || text.contains("media") || text.contains("player") || text.contains("vlc")
        || text.contains("mpv") || text.contains("spotify") || text.contains("sound")
        || text.contains("record") || text.contains("camera") || text.contains("image")
        || text.contains("gimp") || text.contains("inkscape") || text.contains("photo")
        || text.contains("obs") || text.contains("cava") {
        return "Multimedia";
    }

    if text.contains("office") || text.contains("writer") || text.contains("calc")
        || text.contains("impress") || text.contains("document") || text.contains("pdf")
        || text.contains("evince") || text.contains("sheet") || text.contains("word")
        || text.contains("presentation") || text.contains("text editor") || text.contains("notes")
        || text.contains("libreoffice") {
        return "Office";
    }

    if text.contains("system") || text.contains("setting") || text.contains("config")
        || text.contains("control") || text.contains("monitor") || text.contains("btop")
        || text.contains("htop") || text.contains("disk") || text.contains("file")
        || text.contains("nautilus") || text.contains("thunar") || text.contains("dolphin")
        || text.contains("hardware") || text.contains("kernel") || text.contains("package")
        || text.contains("amane") || text.contains("install") {
        return "System";
    }

    "Utilities"
}

fn launch_first_result() {
    let overlay = Overlay::read();
    let apps = filtered_apps(&overlay);
    drop(overlay);

    if let Some(first) = apps.first() {
        let all_apps = Apps::read();
        if let Some(app) = all_apps.list().get(first.index) {
            app.launch();
        }
        Overlay::toggle_app_menu(None);
    }
}

pub fn key_pressed(key: Key) {
    if !Overlay::read().app_menu.shown {
        return;
    }

    match key {
        Key::Escape => {
            let mut overlay = Overlay::write();
            if !overlay.app_menu_query.is_empty() {
                overlay.app_menu_query.clear();
                overlay.app_menu_scroll = 0.0;
                TextInput::set_text(INPUT, "");
            } else {
                overlay.app_menu.hide();
            }
        }
        Key::Enter => {
            launch_first_result();
        }
        _ => {}
    }
}

pub fn ipc(arguments: &[String]) -> String {
    let command = arguments.first().map(String::as_str).unwrap_or("toggle");
    let monitor_arg = arguments.get(1).map(String::as_str);

    match command {
        "show" => {
            let mut overlay = Overlay::write();
            overlay.set_active_monitor(monitor_arg);
            overlay.app_menu_query.clear();
            overlay.app_menu_category = 0;
            overlay.app_menu_scroll = 0.0;
            TextInput::set_text(INPUT, "");
            overlay.app_menu.show();
        }
        "hide" => Overlay::write().app_menu.hide(),
        _ => Overlay::toggle_app_menu(monitor_arg),
    }

    String::from("ok")
}
