mod card;
mod state;

use amane::{
    Center, Color, Column, Full, Key, Keyboard, Layer, LayerWindow, Monitor, Parent,
    Pointer, Rectangle, Service, Stack, Start, Text, Widget, Zone, children,
};

use card::Place;

use super::Wallpaper;
use crate::bar;
use crate::fonts;
use crate::settings::Settings;
use crate::theme;

pub use state::{Picker, list_folder};

// Dimensions for the dropdown wallpaper switcher attached to the top-center pulldown
const PANEL_WIDTH: f32 = 820.0;
const PANEL_HEIGHT: f32 = 360.0;
const RADIUS: f32 = 28.0;
const TOP_OFFSET: f32 = 8.0;
const HIDDEN_MARGIN: f32 = 32.0;

// Thumbnail & card size
pub const CARD_WIDTH: f32 = 260.0;
pub const CARD_HEIGHT: f32 = 160.0;
const CARD_SHARE: f32 = 0.34;

const SPACING: f32 = 215.0;
const CUTOFF_DISTANCE: f32 = 1.25;

const TINT: f32 = 0.48;

/*
 * the wallpaper behind the panel, blurred and tinted, with the wallpapers as
 * cards sliding along it; floating dropdown attached below the top center pill
 */
pub fn view(monitor: &Monitor) -> LayerWindow {
    let picker = Picker::read();

    let opacity = picker.opacity.value();

    let visible = picker.shown || opacity > 0.001;

    if !visible {
        return LayerWindow::new()
            .width(Full)
            .height(Full)
            .layer(Layer::Overlay)
            .space(Zone::Ignore)
            .keyboard(Keyboard::None)
            .namespace("wallpaper-picker")
            .visible(false);
    }

    let theme = theme::current();

    let screen_width = monitor.width as f32;
    let screen_height = monitor.height as f32;

    let panel_width = PANEL_WIDTH.min(screen_width - 48.0);
    let panel_height = PANEL_HEIGHT.min(screen_height - 32.0);

    let keyboard = if picker.shown {
        Keyboard::Exclusive
    } else {
        Keyboard::None
    };

    let opaque = Settings::read().flag("reduce_transparency");

    let backdrop = Rectangle::new()
        .width(panel_width)
        .height(panel_height)
        .radius(RADIUS)
        .fill(super::blurred(screen_width, screen_height));

    // an alpha in the color, since fading the rectangle would give it a canvas of its own
    let tint = if opaque { 1.0 } else { TINT };

    let tint_alpha = (tint * 255.0).round() as u8;

    let surface = theme.surface;

    let tint = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .radius(RADIUS)
        .fill(Color::rgba(surface.red(), surface.green(), surface.blue(), tint_alpha));

    let content: Box<dyn Widget> = if picker.files.is_empty() {
        Box::new(empty(&theme))
    } else {
        Box::new(carousel(&picker, &theme, panel_width, panel_height))
    };

    let band = Stack::new(vec![Box::new(backdrop), Box::new(tint), content])
        .width(panel_width)
        .height(panel_height);

    let top_y = bar::reserved() + TOP_OFFSET;
    let hidden = -(panel_height + HIDDEN_MARGIN);
    let slide_y = top_y + (hidden - top_y) * (1.0 - opacity);

    let band_wrapper = Rectangle::new()
        .width(panel_width)
        .height(panel_height)
        .radius(RADIUS)
        .translate(0.0, slide_y)
        .opacity(opacity)
        .on_click(|_| {})
        .child(band);

    let scrim_alpha = (opacity * 55.0).round() as u8;

    let root = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .align_child(Center, Start)
        .fill(Color::rgba(0, 0, 0, scrim_alpha))
        .on_click(|_| Picker::write().hide())
        .on_scroll(|scroll| {
            if scroll.y > 0.0 || scroll.x > 0.0 {
                Picker::write().slide(1);
            } else if scroll.y < 0.0 || scroll.x < 0.0 {
                Picker::write().slide(-1);
            }
        })
        .child(band_wrapper);

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Overlay)
        .space(Zone::Ignore)
        .keyboard(keyboard)
        .namespace("wallpaper-picker")
        .visible(visible)
        .on_key(key_pressed)
        .child(root)
}

/*
 * like a carousel on a straight line: cards are spread evenly along it,
 * each gets smaller and darker the further it is from the middle
 */
fn carousel(picker: &Picker, theme: &theme::Theme, width: f32, height: f32) -> Stack {
    let count = picker.files.len() as f32;

    let card_width = CARD_WIDTH.min(width * CARD_SHARE);
    let card_height = CARD_HEIGHT.min(height - 40.0);

    let position = picker.position.value();

    let mut places = Vec::new();

    for (index, path) in picker.files.iter().enumerate() {
        // how far from the middle, the short way round
        let distance = (index as f32 - position + count / 2.0).rem_euclid(count) - count / 2.0;

        if distance.abs() > CUTOFF_DISTANCE {
            continue;
        }

        let prominence = (1.0 - distance.abs() / CUTOFF_DISTANCE).clamp(0.0, 1.0);

        let is_center = distance.abs() < 0.45;
        let offset = if is_center {
            0
        } else if distance > 0.0 {
            (distance.round() as i64).max(1)
        } else {
            (distance.round() as i64).min(-1)
        };

        let place = Place {
            x: width / 2.0 + distance * SPACING,
            y: height / 2.0,
            width: card_width,
            height: card_height,
            prominence,
            offset,
            is_center,
        };

        places.push((path, place));
    }

    // the middle card is drawn last, so it lies on top
    places.sort_by(|first, second| first.1.prominence.total_cmp(&second.1.prominence));

    let mut cards: Vec<Box<dyn Widget>> = Vec::new();

    for (path, place) in places {
        cards.push(Box::new(card::view(path, place)));
    }

    // Add navigation chevrons on the sides of the panel
    let btn_bg = Color::rgba(theme.surface.red(), theme.surface.green(), theme.surface.blue(), 200);

    let left_btn = Rectangle::new()
        .width(36.0)
        .height(36.0)
        .radius(18.0)
        .fill(btn_bg)
        .align_child(Center, Center)
        .translate(20.0, height / 2.0 - 18.0)
        .cursor(Pointer)
        .on_click(|_| Picker::write().slide(-1))
        .child(
            Text::new("<")
                .size(18.0)
                .font(fonts::BODY)
                .color(theme.text),
        );

    let right_btn = Rectangle::new()
        .width(36.0)
        .height(36.0)
        .radius(18.0)
        .fill(btn_bg)
        .align_child(Center, Center)
        .translate(width - 20.0 - 36.0, height / 2.0 - 18.0)
        .cursor(Pointer)
        .on_click(|_| Picker::write().slide(1))
        .child(
            Text::new(">")
                .size(18.0)
                .font(fonts::BODY)
                .color(theme.text),
        );

    cards.push(Box::new(left_btn));
    cards.push(Box::new(right_btn));

    Stack::new(cards).width(Parent).height(Parent)
}

fn empty(theme: &theme::Theme) -> Rectangle {
    let title = Text::new("No wallpapers found")
        .size(16.0)
        .font(fonts::BODY)
        .color(theme.text);

    let folder = Text::new(state::folder())
        .size(11.0)
        .font(fonts::BODY)
        .color(theme.muted_text);

    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .align_child(Center, Center)
        .child(Column::new(children![title, folder]).gap(8.0))
}

pub fn ipc(arguments: &[String]) -> String {
    let command = arguments.first().map(String::as_str).unwrap_or("toggle");

    let current = Wallpaper::read().path.clone();

    let mut picker = Picker::write();

    match command {
        "show" => picker.show(&current),
        "hide" => picker.hide(),
        "next" => {
            if !picker.shown {
                picker.show(&current);
            }
            picker.slide(1);
        }
        "prev" => {
            if !picker.shown {
                picker.show(&current);
            }
            picker.slide(-1);
        }
        _ if picker.shown => picker.hide(),
        _ => picker.show(&current),
    }

    String::from("ok")
}

fn key_pressed(key: Key) {
    let mut picker = Picker::write();

    if !picker.shown {
        return;
    }

    match key {
        Key::Left | Key::Down | Key::Character('h') | Key::Character('j') => picker.slide(-1),
        Key::Right | Key::Up | Key::Tab | Key::Character('l') | Key::Character('k') => picker.slide(1),
        Key::Escape => picker.hide(),

        Key::Enter | Key::Space => {
            if let Some(path) = picker.selected() {
                super::choose(path);
            }

            picker.hide();
        }

        _ => {}
    }
}
