mod app_icon;
mod bar;
mod clock;
mod converter;
mod floating;
mod fonts;
mod integrations;
mod liquid;
mod lock_screen;
mod mixer;
mod monitors;
mod motion;
mod overlay;
mod pomodoro;
mod recorder;
mod profile;
mod screen_mask;
mod settings;
mod shake_cursor;
mod theme;
mod wallpaper;

use amane::{App, Apps, Notifications, Service};

use integrations::Export;
use lock_screen::Logind;
use wallpaper::{Shuffle, Wallpaper};

fn ipc_monitor_layout(_arguments: &[String]) -> String {
    settings::open_on(settings::Page::Monitors);
    String::from("ok")
}

fn ipc_brightness(arguments: &[String]) -> String {
    let arg = arguments.first().map(String::as_str).unwrap_or("get");
    let current = amane::Brightness::read().percent() as i32;

    if arg == "get" {
        return current.to_string();
    }

    let target = if let Some(inc) = arg.strip_prefix('+') {
        let delta = inc.trim_end_matches('%').parse::<i32>().unwrap_or(10);
        (current + delta).clamp(1, 100) as u8
    } else if let Some(dec) = arg.strip_suffix('-') {
        let delta = dec.trim_end_matches('%').parse::<i32>().unwrap_or(10);
        (current - delta).clamp(1, 100) as u8
    } else if let Some(dec) = arg.strip_prefix('-') {
        let delta = dec.trim_end_matches('%').parse::<i32>().unwrap_or(10);
        (current - delta).clamp(1, 100) as u8
    } else {
        arg.trim_end_matches('%').parse::<u8>().unwrap_or(current as u8).clamp(1, 100)
    };

    amane::Brightness::set(target);
    target.to_string()
}

fn main() {
    // reading it once starts the palette before the first frame
    drop(Wallpaper::read());

    // counts down to the next random wallpaper while shuffle is on
    drop(Shuffle::read());

    // the app list is read once up front, so the launcher opens without waiting for it
    drop(Apps::read());

    // the notification server starts here, so nothing sent before the panel first opens is lost
    drop(Notifications::read());

    // listens for logind asking to lock, like `loginctl lock-session` from the power menu
    drop(Logind::read());

    // writes the theme out to the programs turned on under integrations
    drop(Export::read());

    // starts COSMIC settings daemon bridge for iced/cosmic apps live reloading
    integrations::cosmic::start_daemon();

    // pre-initialize background services for system monitor, power devices, and audio control
    drop(overlay::sysmon::SysmonData::read());
    drop(overlay::power_devices::PowerDevicesData::read());
    drop(overlay::audio_control::AudioDevices::read());

    // shake-to-find cursor background service
    drop(shake_cursor::ShakeCursor::read());

    // monitor manager and safety guard
    drop(monitors::MonitorManager::read());

    // window layout tracker for floating desktop widgets adjustment
    drop(floating::windows::DesktopWindows::read());

    // initialize blue light nightlight filter if previously enabled
    overlay::settings_menu::init_nightlight();


    App::new()
        .font(fonts::body())
        .window_per_monitor(wallpaper::view)
        // over the wallpaper, under everything else
        .window_per_monitor(floating::view)
        // made before the bar and panels, so it sits under them in the same layer
        .window_per_monitor(screen_mask::view)
        .window_per_monitor(bar::view)
        // under the panels, catching clicks outside them when that setting is on
        .window_per_monitor(overlay::dismiss::view)
        .window_per_monitor(overlay::view)
        .window_per_monitor(wallpaper::picker::view)
        .lock(lock_screen::view)
        .ipc("brightness", ipc_brightness)
        .ipc("launcher", overlay::launcher::ipc)
        .ipc("app_menu", overlay::app_menu::ipc)
        .ipc("settings_menu", overlay::settings_menu::ipc)
        .ipc("power_menu", overlay::power_menu::ipc)
        .ipc("utility", overlay::utility::ipc)
        .ipc("control", overlay::control_center::ipc)
        .ipc("calendar", overlay::control_center::calendar::ipc)
        .ipc("sysmon", overlay::sysmon::ipc)
        .ipc("power", overlay::power_devices::ipc)
        .ipc("audio_control", overlay::audio_control::ipc)
        .ipc("home_assistant", overlay::home_assistant::ipc)
        .ipc("monitor_layout", ipc_monitor_layout)
        .ipc("monitors", monitors::ipc)
        .ipc("settings", settings::ipc)
        .ipc("lock", lock_screen::ipc)
        .ipc("wallpaper", wallpaper::picker::ipc)
        .ipc("converter", converter::ipc)
        .ipc("mixer", mixer::ipc)
        .ipc("record", recorder::ipc)
        .run();
}
