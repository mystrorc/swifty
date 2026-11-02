use std::time::Duration;

use amane::{
    Center, Column, End, Image, Media, MediaPlayer, Padding, Pointer, Rectangle, Row,
    Service, SpaceBetween, Stack, Start, Text, Weight, Widget, children,
};

use super::{art, audio_eq, player, visualizer, wave};
use crate::fonts;
use crate::motion::{self, FAST_SPATIAL};
use crate::overlay::Overlay;
use crate::overlay::utility::{fade_target, hover};
use crate::theme::{self, Theme};

const RADIUS: f32 = 20.0;
const ART_RADIUS: f32 = 14.0;

const PADDING: f32 = 14.0;
const GAP: f32 = 14.0;
const LINE_GAP: f32 = 6.0;

const TIME_WIDTH: f32 = 30.0;
const CONTROLS_HEIGHT: f32 = 38.0;

const BUTTON: f32 = 32.0;
const PLAY_BUTTON: f32 = 40.0;

const EMPTY_ICON: &str = "󰝚";
const PREVIOUS_ICON: &str = "󰒮";
const PLAY_ICON: &str = "󰐊";
const PAUSE_ICON: &str = "󰏤";
const NEXT_ICON: &str = "󰒭";

// with several players open, a switch arrow sits on either side of the card
const ARROW_WIDTH: f32 = 26.0;
const ARROW_INSET: f32 = 8.0;
const ARROWS_PADDING: f32 = 38.0;

// how far a switched card slides in from
const SWITCH_DISTANCE: f32 = 28.0;

const PREVIOUS_PLAYER_ICON: &str = "󰁍";
const NEXT_PLAYER_ICON: &str = "󰁔";

// Combined view: Now Playing on top + 10-Band Graphic Equalizer & Presets on bottom
pub fn view(overlay: &Overlay, theme: &Theme, width: f32, height: f32) -> Stack {
    let media = Media::read();
    let player = player::shown(&media, overlay);
    let several = media.players().len() > 1;

    let side_padding = if several { ARROWS_PADDING } else { PADDING };

    let inner_width = width - side_padding * 2.0;
    let inner_height = height - PADDING * 2.0;

    let now_playing_height: f32 = 195.0;
    let divider_height: f32 = 1.0;
    let gap: f32 = 10.0;
    let eq_height: f32 = (inner_height - now_playing_height - divider_height - gap * 2.0).max(120.0);

    let art_size = 110.0_f32.min(now_playing_height);
    let details_width = inner_width - art_size - GAP;

    let switch = overlay.player_switch.value();

    let row = Row::new(children![
        cover(player, theme, art_size),
        details(overlay, player, theme, details_width, now_playing_height),
    ])
    .gap(GAP)
    .align(Center);

    let content = Rectangle::new()
        .width(inner_width)
        .height(now_playing_height)
        .translate(switch * SWITCH_DISTANCE, 0.0)
        .opacity(1.0 - switch.abs())
        .align_child(Start, Center)
        .child(row);

    let sep = Rectangle::new()
        .width(inner_width)
        .height(divider_height)
        .fill(theme::with_opacity(theme.border, 0.45));

    let eq_view = eq_section(theme, inner_width, eq_height);

    let col = Column::new(children![content, sep, eq_view]).gap(gap);

    let card = Rectangle::new()
        .width(width)
        .height(height)
        .radius(RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: PADDING,
            right: side_padding,
            bottom: PADDING,
            left: side_padding,
        })
        .child(col);

    let mut layers: Vec<Box<dyn Widget>> = vec![Box::new(card)];

    if several {
        layers.push(Box::new(arrow(overlay, theme, now_playing_height, -1, ARROW_INSET)));
        layers.push(Box::new(arrow(overlay, theme, now_playing_height, 1, width - ARROW_INSET - ARROW_WIDTH)));
    }

    Stack::new(layers).width(width).height(height)
}

fn eq_section(theme: &Theme, width: f32, height: f32) -> Column {
    let eq = audio_eq::EqData::read();

    let top_bar = Row::new(children![
        audio_eq::preset_selector(theme, &eq.active_preset, 240.0),
        audio_eq::bypass_button(theme, eq.is_bypassed),
    ])
    .width(width)
    .justify(SpaceBetween)
    .align(Center);

    let bands_grid = audio_eq::bands_row(theme, &eq, width, height - 28.0);

    Column::new(children![top_bar, bands_grid])
        .width(width)
        .gap(4.0)
}

// a tall button at the card's edge that shows the player before or after this one
fn arrow(overlay: &Overlay, theme: &Theme, height: f32, direction: i32, x: f32) -> Rectangle {
    let (name, icon) = if direction < 0 {
        ("media:previous-player", PREVIOUS_PLAYER_ICON)
    } else {
        ("media:next-player", NEXT_PLAYER_ICON)
    };

    let hover_name = String::from(name);

    let amount = motion::fade(name, fade_target(overlay, name));

    let fill = theme::mix(theme.selected_surface, theme.border, amount);

    Rectangle::new()
        .width(ARROW_WIDTH)
        .height(height)
        .radius(10.0)
        .fill(fill)
        .translate(x, PADDING)
        .cursor(Pointer)
        .on_hover(move |inside| hover(hover_name.clone(), inside))
        .on_click(move |_| player::switch(direction))
        .align_child(Center, Center)
        .child(Text::new(icon).size(14.0).font(fonts::NERD).tight().color(theme.text))
}

// a music note stands in until the cover is on disk and decoded
fn cover(player: Option<&MediaPlayer>, theme: &Theme, size: f32) -> Rectangle {
    let slot = Rectangle::new()
        .width(size)
        .height(size)
        .radius(ART_RADIUS)
        .fill(theme.border)
        .clip();

    let url = player.map_or("", MediaPlayer::art_url);

    if let Some(path) = art::path(url) {
        if Image::loaded(&path) {
            let pixels = size as u32;

            return slot.fill(Image::cover(path).thumbnail(pixels, pixels));
        }
    }

    slot.align_child(Center, Center)
        .child(Text::new(EMPTY_ICON).size(42.0).font(fonts::NERD).tight().color(theme.muted_text))
}

fn details(
    overlay: &Overlay,
    player: Option<&MediaPlayer>,
    theme: &Theme,
    width: f32,
    height: f32,
) -> Column {
    let (title, artist) = match player {
        Some(player) if !player.title().is_empty() => (player.title(), player.artist()),
        _ => ("Nothing playing", "Open Spotify or another media player"),
    };

    let title_h = 22.0;
    let artist_h = 16.0;

    let title = line(title, width, title_h, 15.0, Weight::Bold, theme.text);
    let artist = line(artist, width, artist_h, 12.0, Weight::Regular, theme.muted_text);

    let fixed = title_h
        + artist_h
        + visualizer::HEIGHT
        + wave::HEIGHT
        + CONTROLS_HEIGHT
        + LINE_GAP * 4.0;

    let space = Rectangle::new().width(width).height((height - fixed).max(0.0));

    Column::new(children![
        title,
        artist,
        space,
        visualizer::view(theme, width),
        progress(player, theme, width),
        controls(overlay, player, theme, width),
    ])
    .gap(LINE_GAP)
}

// one line of text, cut off with an ellipsis when too long
fn line(text: &str, width: f32, height: f32, size: f32, weight: Weight, color: amane::Color) -> Rectangle {
    Rectangle::new()
        .width(width)
        .height(height)
        .align_child(Start, Center)
        .child(Text::new(text).size(size).font(fonts::BODY).weight(weight).color(color).elide())
}

fn progress(player: Option<&MediaPlayer>, theme: &Theme, width: f32) -> Row {
    let position = player.map_or(Duration::ZERO, MediaPlayer::position);
    let length = player.map_or(Duration::ZERO, MediaPlayer::length);

    let playing = player.is_some_and(MediaPlayer::playing);

    let played = if length.is_zero() {
        0.0
    } else {
        position.as_secs_f32() / length.as_secs_f32()
    };

    let wave_width = width - TIME_WIDTH * 2.0 - LINE_GAP * 2.0;

    Row::new(children![
        time(position, theme, Start),
        wave::view(wave_width, played, playing, theme.accent, theme.border),
        time(length, theme, End),
    ])
    .gap(LINE_GAP)
    .align(Center)
}

// "3:07"
fn time(duration: Duration, theme: &Theme, side: impl Into<amane::Align>) -> Rectangle {
    let seconds = duration.as_secs();

    let text = format!("{}:{:02}", seconds / 60, seconds % 60);

    Rectangle::new()
        .width(TIME_WIDTH)
        .height(wave::HEIGHT)
        .align_child(side, Center)
        .child(Text::new(text).size(10.0).font(fonts::BODY).color(theme.muted_text))
}

fn controls(overlay: &Overlay, player: Option<&MediaPlayer>, theme: &Theme, width: f32) -> Row {
    let playing = player.is_some_and(MediaPlayer::playing);

    let play_icon = if playing { PAUSE_ICON } else { PLAY_ICON };

    // the buttons act on the player shown, looked up again by name when clicked
    let name = player.map_or("", MediaPlayer::name);

    let buttons: Vec<Box<dyn Widget>> = vec![
        Box::new(button(overlay, theme, "media:previous", PREVIOUS_ICON, name, MediaPlayer::previous)),
        Box::new(play_button(playing, theme, play_icon, name)),
        Box::new(button(overlay, theme, "media:next", NEXT_ICON, name, MediaPlayer::next)),
    ];

    Row::new(buttons)
        .width(width)
        .height(CONTROLS_HEIGHT)
        .gap(10.0)
        .justify(Center)
        .align(Center)
}

// a round button that only shows a background under the pointer
fn button(
    overlay: &Overlay,
    theme: &Theme,
    hover_name: &str,
    icon: &str,
    player_name: &str,
    action: fn(&MediaPlayer),
) -> Rectangle {
    let amount = motion::fade(hover_name, fade_target(overlay, hover_name));

    let player_name = String::from(player_name);
    let hover_name = String::from(hover_name);

    // the card's own color, since mix gives an opaque color and transparent would turn black
    let fill = theme::mix(theme.surface, theme.border, amount);

    Rectangle::new()
        .width(BUTTON)
        .height(BUTTON)
        .radius(BUTTON / 2.0)
        .fill(fill)
        .cursor(Pointer)
        .on_hover(move |inside| hover(hover_name.clone(), inside))
        .on_click(move |_| player::control(&player_name, action))
        .align_child(Center, Center)
        .child(Text::new(icon).size(16.0).font(fonts::NERD).tight().color(theme.text))
}

// round while paused, and squares off a little while playing
fn play_button(playing: bool, theme: &Theme, icon: &str, player_name: &str) -> Rectangle {
    let player_name = String::from(player_name);

    let target = if playing { 10.0 } else { PLAY_BUTTON / 2.0 };

    let radius = motion::follow("media:play-radius", target, FAST_SPATIAL);

    Rectangle::new()
        .width(PLAY_BUTTON)
        .height(PLAY_BUTTON)
        .radius(radius)
        .fill(theme.accent)
        .cursor(Pointer)
        .on_click(move |_| player::control(&player_name, MediaPlayer::play_pause))
        .align_child(Center, Center)
        .child(Text::new(icon).size(19.0).font(fonts::NERD).tight().color(theme.on_accent))
}
