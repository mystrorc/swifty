use super::super::choice;
use super::super::field;
use super::super::page::Page;
use super::super::slider::{self, Range};
use super::super::switch;

pub const CURSORS: [(&str, &str); 10] = [
    ("Adwaita (Default)", "Adwaita"),
    ("Bibata Modern Classic", "Bibata-Modern-Classic"),
    ("Bibata Modern Ice", "Bibata-Modern-Ice"),
    ("Bibata Modern Amber", "Bibata-Modern-Amber"),
    ("Bibata Original Classic", "Bibata-Original-Classic"),
    ("Bibata Original Ice", "Bibata-Original-Ice"),
    ("Catppuccin Mocha Dark", "catppuccin-mocha-dark-cursors"),
    ("Catppuccin Latte Light", "catppuccin-latte-light-cursors"),
    ("Nordzy Cursors", "Nordzy-cursors"),
    ("Pop OS", "Pop"),
];

pub const SIZES: [(&str, &str); 4] = [
    ("Small (24px)", "24"),
    ("Medium (32px)", "32"),
    ("Large (48px)", "48"),
    ("Extra Large (64px)", "64"),
];

pub const GLOW_MODES: [(&str, &str); 2] = [
    ("Match theme accent", "theme"),
    ("Pick a color", "custom"),
];

pub const GLOW_COLORS: [(&str, &str); 9] = [
    ("Neon Cyan", "#00d2ff"),
    ("Electric Blue", "#3b82f6"),
    ("Neon Pink", "#ff2a85"),
    ("Neon Purple", "#a855f7"),
    ("Emerald Green", "#10b981"),
    ("Amber Gold", "#f59e0b"),
    ("Crimson Red", "#ef4444"),
    ("Cyber Yellow", "#eab308"),
    ("Arctic White", "#f8fafc"),
];

pub const GLOW_SIZES: [(&str, &str); 3] = [
    ("Subtle (3px)", "3"),
    ("Medium (5px)", "5"),
    ("Vibrant (8px)", "8"),
];

pub const FONTS: [(&str, &str); 10] = [
    ("Poppins (Default)", "Poppins"),
    ("Inter", "Inter"),
    ("Adwaita Sans", "Adwaita Sans"),
    ("Cantarell", "Cantarell"),
    ("JetBrains Mono", "JetBrainsMono Nerd Font"),
    ("Fira Sans", "Fira Sans"),
    ("Roboto", "Roboto"),
    ("Ubuntu", "Ubuntu"),
    ("Noto Sans", "Noto Sans"),
    ("DejaVu Sans", "DejaVu Sans"),
];

pub fn build(page: &mut Page) {
    // Window Appearance (Corners, Borders, Gaps, Shadows)
    slider::add(
        page,
        "window_corner_radius",
        "Window corner radius",
        "Rounded corner radius clipped across all windows and borders",
        Range {
            min: 0.0,
            max: 24.0,
            step: 2.0,
            label: slider::pixels,
        },
    );

    slider::add(
        page,
        "window_border_width",
        "Window border width",
        "Thickness of the rounded border around windows",
        Range {
            min: 0.0,
            max: 6.0,
            step: 1.0,
            label: slider::pixels,
        },
    );

    switch::add(
        page,
        "window_inactive_border",
        "Inactive window border",
        "Show a subtle rounded border around unfocused windows",
    );

    slider::add(
        page,
        "window_gaps",
        "Window gaps",
        "Spacing between tiled windows on the desktop",
        Range {
            min: 0.0,
            max: 24.0,
            step: 2.0,
            label: slider::pixels,
        },
    );

    switch::add(
        page,
        "window_shadows",
        "Window drop shadows",
        "Render soft rounded drop shadows behind windows",
    );

    page.end_group();

    // Typography & Cursors
    choice::add(
        page,
        "font_family",
        "Font family",
        "Primary typeface used across the shell",
        &FONTS,
    );

    choice::add(
        page,
        "cursor_theme",
        "Cursor theme",
        "Active mouse pointer and cursor theme",
        &CURSORS,
    );

    choice::add(
        page,
        "cursor_size",
        "Cursor size",
        "Scale of the mouse pointer in pixels",
        &SIZES,
    );

    switch::add(
        page,
        "shake_cursor",
        "Shake to find cursor",
        "Temporarily enlarge the cursor when shaken rapidly",
    );

    switch::add(
        page,
        "cursor_glow",
        "Cursor glow",
        "Render a luminous colored glow halo around the cursor pointer",
    );

    choice::add(
        page,
        "cursor_glow_mode",
        "Glow color mode",
        "Match desktop theme accent or pick a custom color",
        &GLOW_MODES,
    );

    choice::add(
        page,
        "cursor_glow_color",
        "Glow color",
        "Color of the cursor glow halo when custom mode is chosen",
        &GLOW_COLORS,
    );

    choice::add(
        page,
        "cursor_glow_radius",
        "Glow radius",
        "Spread and size of the luminous cursor glow halo",
        &GLOW_SIZES,
    );

    // Wallpaper blur and transparency
    slider::add(
        page,
        "blur_strength",
        "Blur strength",
        "Control wallpaper blur behind the wallpaper picker",
        Range {
            min: 0.0,
            max: 1.0,
            step: 0.05,
            label: slider::plain,
        },
    );

    switch::add(
        page,
        "reduce_transparency",
        "Reduce transparency",
        "Solid wallpaper picker and app backgrounds with no blur",
    );

    page.end_group();

    // Wallpaper & Transitions
    choice::add(
        page,
        "wallpaper_transition",
        "Wallpaper transition",
        "Choose how the new wallpaper replaces the current one",
        &[("Circle", "circle"), ("Fade", "fade"), ("Instant", "instant")],
    );

    slider::add(
        page,
        "wallpaper_duration",
        "Transition duration",
        "Duration used by wallpaper reveal and fade animations",
        Range {
            min: 100.0,
            max: 3000.0,
            step: 100.0,
            label: slider::milliseconds,
        },
    );

    field::add(
        page,
        "wallpaper_folder",
        "Wallpaper directory",
        "Folder scanned by the wallpaper picker and shuffle timer",
    );

    switch::add(
        page,
        "wallpaper_shuffle",
        "Random wallpaper",
        "Automatically choose another image from the wallpaper directory",
    );

    slider::add(
        page,
        "wallpaper_shuffle_minutes",
        "Shuffle interval",
        "How often a random wallpaper is selected",
        Range {
            min: 1.0,
            max: 180.0,
            step: 1.0,
            label: slider::minutes,
        },
    );

    switch::add(
        page,
        "follow_wallpaper",
        "Update colors with wallpaper",
        "Rebuild the dynamic palette after a wallpaper change",
    );

    page.end_group();

    // Motion & Animations
    slider::add(
        page,
        "animation_speed",
        "Animation speed",
        "Apply a global speed multiplier to every animation",
        Range {
            min: 0.5,
            max: 2.0,
            step: 0.1,
            label: slider::times,
        },
    );

    switch::add(
        page,
        "reduce_motion",
        "Reduce motion",
        "Disable shell motion while preserving state changes",
    );

    page.end_group();
}

