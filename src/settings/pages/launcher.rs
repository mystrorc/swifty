use super::super::page::Page;
use super::super::slider::{self, Range};
use super::super::switch;

pub fn build(page: &mut Page) {
    slider::add(
        page,
        "launcher_width",
        "Launcher width",
        "Set the preferred launcher panel width",
        Range {
            min: 420.0,
            max: 900.0,
            step: 20.0,
            label: slider::pixels,
        },
    );

    slider::add(
        page,
        "launcher_rows",
        "Visible results",
        "Limit how many rows are shown before the list scrolls",
        Range {
            min: 3.0,
            max: 14.0,
            step: 1.0,
            label: slider::count,
        },
    );

    switch::add(
        page,
        "launcher_descriptions",
        "Application descriptions",
        "Show generic names or application comments below results",
    );

    switch::add(page, "launcher_icons", "Result icons", "Show application and command icons");

    switch::add(
        page,
        "launcher_remember_query",
        "Remember query",
        "Keep the last search when the launcher is reopened",
    );

    page.end_group();

    // Elephant Backend Integration
    switch::add(
        page,
        "launcher_elephant",
        "Elephant backend",
        "Integrate Elephant daemon for calculator, open windows, and clipboard",
    );
    switch::add(
        page,
        "launcher_elephant_calc",
        "Calculator",
        "Calculate math expressions right in search (e.g. 2+2, 12*15)",
    );
    switch::add(
        page,
        "launcher_elephant_windows",
        "Window switcher",
        "Search and focus open application windows",
    );
    switch::add(
        page,
        "launcher_elephant_clipboard",
        "Clipboard history",
        "Access clipboard history with c or clip: prefix",
    );

    page.end_group();

    switch::add(page, "launcher_commands", "Command mode", "Enable the > command palette prefix");
    switch::add(page, "command_settings", "Settings command", "Show Settings in command mode");
    switch::add(page, "command_colors", "Color scheme command", "Show Color scheme in command mode");
    switch::add(page, "command_tmux", "Tmux command", "Show Tmux sessions in command mode");
    switch::add(page, "command_wallpapers", "Wallpaper command", "Show Wallpapers in command mode");
    switch::add(page, "command_converter", "Converter command", "Show Converter in command mode");
    switch::add(page, "command_pomodoro", "Pomodoro command", "Show Pomodoro in command mode");

    page.end_group();

    // Dismissal & Behavior
    switch::add(
        page,
        "launcher_close_on_launch",
        "Close launcher after launch",
        "Dismiss the launcher after starting an app or tmux session",
    );

    switch::add(
        page,
        "launcher_escape_clears",
        "Escape clears launcher first",
        "Clear a non-empty query before Escape closes the launcher",
    );

    switch::add(
        page,
        "click_outside_dismiss",
        "Click outside to dismiss",
        "Let clicks outside open panels close the current overlay",
    );
    switch::add(
        page,
        "command_mixer",
        "Mixer command",
        "Show Mixer in command mode",
    );

    page.end_group();
}
