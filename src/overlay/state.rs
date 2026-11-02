use amane::{Notification, Notifications, Service};

use super::panel::Panel;
use super::utility::{self, Page};
use crate::motion::{self, Glide};

// how long the power menu's hover fill and the launcher's list take to move
const FILL_DURATION: u64 = 180;
pub const LIST_DURATION: u64 = 160;

/*
 * the state every overlay panel shares, so opening one closes the others
 * without having to pass handles around; changed from inside widgets
 */
pub struct Overlay {
    pub power_menu: Panel,

    // how far each action has grown its hover background, 0 to 1
    pub action_fills: Vec<Glide>,

    // whether the pointer is over the top bar
    pub bar_hovered: bool,

    pub control_center: Panel,

    // higher-level tab in the middle pulldown: 0 = System Monitor, 1 = Media, 2 = Calendar, 3 = Battery
    pub center_tab: usize,

    // sub-tab under Media: 0 = Now Playing, 1 = Audio & EQ
    pub control_tab: usize,

    pub launcher: Panel,
    pub launcher_monitor: Option<String>,
    pub active_monitor: Option<String>,

    pub app_menu: Panel,
    pub app_menu_query: String,
    pub app_menu_category: usize,
    pub app_menu_scroll: f32,
    pub app_menu_hovered: Option<usize>,

    pub settings_menu: Panel,
    pub settings_tab: usize,

    pub sysmon: Panel,

    pub power_devices: Panel,

    pub audio_control: Panel,
    pub home_assistant: Panel,

    pub query: String,

    // the chosen result, and the first result the list shows
    pub selected: usize,
    pub first: usize,

    pub hovered_row: Option<usize>,

    // which result the highlight is on, and which one is at the top of the list, both sliding
    pub highlight: Glide,
    pub scroll: Glide,

    pub sessions: Vec<String>,

    pub utility: Panel,

    pub page: Page,

    pub appearance_tab: usize,
    pub scheme_dropdown_open: bool,

    // the control under the pointer in the utility center, like "tab:wifi"
    pub hovered: Option<String>,

    // the network whose password is being typed
    pub password_for: Option<String>,
    pub show_password: bool,

    // the month the calendar shows, counted from this month
    pub calendar_month: i64,

    // the network last asked to join, shown as connecting until it is up
    pub joining: Option<String>,

    // the popup under the pointer, whose countdown waits
    // the media player the control center shows, by bus name; none shows the active one
    pub player: Option<String>,

    // slides the media card in from the side it was switched toward, 0 once settled
    pub player_switch: Glide,

    pub hovered_popup: Option<u32>,

    // popups closed by hand, which leave before their time
    pub closed_popups: Vec<u32>,
}

impl Service for Overlay {
    fn new() -> Self {
        let mut action_fills = Vec::new();

        for _ in 0..super::power_menu::ACTION_COUNT {
            action_fills.push(motion::spatial(0.0, FILL_DURATION));
        }

        Self {
            power_menu: Panel::new(),
            action_fills,
            bar_hovered: false,
            control_center: Panel::new(),
            center_tab: 1,
            control_tab: 0,
            launcher: Panel::new(),
            launcher_monitor: None,
            active_monitor: None,
            app_menu: Panel::new(),
            app_menu_query: String::new(),
            app_menu_category: 0,
            app_menu_scroll: 0.0,
            app_menu_hovered: None,
            settings_menu: Panel::new(),
            settings_tab: 0,
            sysmon: Panel::new(),
            power_devices: Panel::new(),
            audio_control: Panel::new(),
            home_assistant: Panel::new(),
            query: String::new(),
            selected: 0,
            first: 0,
            hovered_row: None,
            highlight: motion::spatial(0.0, LIST_DURATION),
            scroll: motion::spatial(0.0, LIST_DURATION),
            sessions: Vec::new(),
            utility: Panel::new(),
            page: Page::Notifications,
            appearance_tab: 0,
            scheme_dropdown_open: false,
            hovered: None,
            password_for: None,
            show_password: false,
            joining: None,
            calendar_month: 0,
            player: None,
            player_switch: motion::spatial(0.0, motion::FAST_SPATIAL),
            hovered_popup: None,
            closed_popups: Vec::new(),
        }
    }

    // only changes through input
    fn listen() {}
}

impl Overlay {
    pub fn focused_monitor() -> Option<String> {
        amane::Workspaces::read()
            .list()
            .iter()
            .find(|w| w.focused())
            .and_then(|w| w.output().map(String::from))
    }

    pub fn set_active_monitor(&mut self, monitor: Option<&str>) {
        if let Some(name) = monitor {
            self.active_monitor = Some(name.to_string());
        } else if let Some(focused) = Self::focused_monitor() {
            self.active_monitor = Some(focused);
        } else if self.active_monitor.is_none() {
            self.active_monitor = amane::Workspaces::read()
                .list()
                .first()
                .and_then(|w| w.output().map(String::from));
        }
        self.launcher_monitor = self.active_monitor.clone();
    }

    // only one panel is out at a time
    pub fn toggle_power_menu(monitor: Option<&str>) {
        let mut overlay = Self::write();

        overlay.launcher.hide();
        overlay.app_menu.hide();
        overlay.settings_menu.hide();
        overlay.control_center.hide();
        overlay.sysmon.hide();
        overlay.power_devices.hide();
        overlay.audio_control.hide();
        overlay.home_assistant.hide();
        utility::close(&mut overlay);

        if overlay.power_menu.shown {
            overlay.power_menu.hide();
        } else {
            overlay.set_active_monitor(monitor);
            overlay.power_menu.show();
        }
    }

    #[allow(dead_code)]
    pub fn hide_power_menu() {
        Self::write().power_menu.hide();
    }

    pub fn toggle_app_menu(monitor: Option<&str>) {
        let mut overlay = Self::write();

        overlay.launcher.hide();
        overlay.power_menu.hide();
        overlay.settings_menu.hide();
        overlay.control_center.hide();
        overlay.sysmon.hide();
        overlay.power_devices.hide();
        overlay.audio_control.hide();
        overlay.home_assistant.hide();
        utility::close(&mut overlay);

        if overlay.app_menu.shown {
            overlay.app_menu.hide();
        } else {
            overlay.set_active_monitor(monitor);
            overlay.app_menu_query.clear();
            overlay.app_menu_category = 0;
            overlay.app_menu_scroll = 0.0;
            overlay.app_menu_hovered = None;
            amane::TextInput::set_text("app_menu_search", "");
            overlay.app_menu.show();
        }
    }

    #[allow(dead_code)]
    pub fn hide_app_menu() {
        Self::write().app_menu.hide();
    }

    pub fn toggle_settings_menu(monitor: Option<&str>) {
        let mut overlay = Self::write();

        overlay.launcher.hide();
        overlay.power_menu.hide();
        overlay.app_menu.hide();
        overlay.control_center.hide();
        overlay.sysmon.hide();
        overlay.power_devices.hide();
        overlay.audio_control.hide();
        overlay.home_assistant.hide();
        utility::close(&mut overlay);

        if overlay.settings_menu.shown {
            overlay.settings_menu.hide();
        } else {
            overlay.set_active_monitor(monitor);
            overlay.settings_menu.show();
        }
    }

    #[allow(dead_code)]
    pub fn hide_settings_menu() {
        Self::write().settings_menu.hide();
    }

    pub fn set_settings_tab(tab: usize) {
        Self::write().settings_tab = tab;
    }

    pub fn toggle_utility(monitor: Option<&str>) {
        let mut overlay = Self::write();

        overlay.launcher.hide();
        overlay.app_menu.hide();
        overlay.settings_menu.hide();
        overlay.power_menu.hide();
        overlay.control_center.hide();
        overlay.sysmon.hide();
        overlay.power_devices.hide();
        overlay.audio_control.hide();
        overlay.home_assistant.hide();

        if overlay.utility.shown {
            utility::close(&mut overlay);
            return;
        }

        overlay.set_active_monitor(monitor);
        overlay.utility.show();
        utility::opened(&mut overlay);
    }

    pub fn set_center_tab(tab: usize) {
        Self::write().center_tab = tab;
    }

    pub fn set_control_tab(tab: usize) {
        let mut overlay = Self::write();
        overlay.center_tab = 1;
        overlay.control_tab = tab;
    }

    pub fn open_control_tab(tab: usize, monitor: Option<&str>) {
        let mut overlay = Self::write();
        overlay.set_active_monitor(monitor);
        overlay.center_tab = 1;
        overlay.control_tab = tab;
        overlay.launcher.hide();
        overlay.app_menu.hide();
        overlay.settings_menu.hide();
        overlay.power_menu.hide();
        overlay.sysmon.hide();
        overlay.power_devices.hide();
        overlay.audio_control.hide();
        overlay.home_assistant.hide();
        utility::close(&mut overlay);
        overlay.control_center.show();
    }

    pub fn toggle_center_section(section: usize, monitor: Option<&str>) {
        let mut overlay = Self::write();

        overlay.launcher.hide();
        overlay.app_menu.hide();
        overlay.settings_menu.hide();
        overlay.power_menu.hide();
        overlay.sysmon.hide();
        overlay.power_devices.hide();
        overlay.audio_control.hide();
        overlay.home_assistant.hide();
        utility::close(&mut overlay);

        if overlay.control_center.shown && overlay.center_tab == section {
            overlay.control_center.hide();
        } else {
            overlay.set_active_monitor(monitor);
            overlay.center_tab = section;
            overlay.control_center.show();
        }
    }

    pub fn toggle_control_center(monitor: Option<&str>) {
        Self::toggle_center_section(1, monitor);
    }

    pub fn toggle_sysmon(monitor: Option<&str>) {
        Self::toggle_center_section(0, monitor);
    }

    pub fn toggle_calendar(monitor: Option<&str>) {
        Self::toggle_center_section(2, monitor);
    }

    pub fn toggle_power_devices(monitor: Option<&str>) {
        Self::toggle_center_section(3, monitor);
    }

    #[allow(dead_code)]
    pub fn hover_power_devices(inside: bool) {
        Self::hover_control_center(inside);
    }

    #[allow(dead_code)]
    pub fn toggle_home_assistant(monitor: Option<&str>) {
        let mut overlay = Self::write();

        overlay.launcher.hide();
        overlay.app_menu.hide();
        overlay.settings_menu.hide();
        overlay.power_menu.hide();
        overlay.control_center.hide();
        overlay.sysmon.hide();
        overlay.power_devices.hide();
        overlay.audio_control.hide();
        overlay.home_assistant.hide();
        utility::close(&mut overlay);

        if overlay.home_assistant.shown {
            overlay.home_assistant.hide();
        } else {
            overlay.set_active_monitor(monitor);
            overlay.home_assistant.show();
        }
    }

    pub fn hover_home_assistant(inside: bool) {
        let mut overlay = Self::write();
        overlay.home_assistant.hovered = inside;
        if inside {
            overlay.home_assistant.was_hovered = true;
        }
        overlay.dismiss_if_left();
    }

    pub fn toggle_audio_control(monitor: Option<&str>) {
        let mut overlay = Self::write();

        overlay.launcher.hide();
        overlay.app_menu.hide();
        overlay.settings_menu.hide();
        overlay.power_menu.hide();
        overlay.control_center.hide();
        overlay.sysmon.hide();
        overlay.power_devices.hide();
        overlay.home_assistant.hide();
        utility::close(&mut overlay);

        if overlay.audio_control.shown {
            overlay.audio_control.hide();
        } else {
            overlay.set_active_monitor(monitor);
            overlay.audio_control.show();
        }
    }

    #[allow(dead_code)]
    pub fn hover_sysmon(inside: bool) {
        Self::hover_control_center(inside);
    }

    pub fn hover_audio_control(inside: bool) {
        {
            let overlay = Self::read();
            if overlay.audio_control.hovered == inside && (!inside || overlay.audio_control.was_hovered) {
                return;
            }
        }
        let mut overlay = Self::write();

        overlay.audio_control.hovered = inside;

        if inside {
            overlay.audio_control.was_hovered = true;
        }

        if !inside && overlay.audio_control.was_hovered {
            overlay.audio_control.hide();
        }
    }

    pub fn hover_control_center(inside: bool) {
        {
            let overlay = Self::read();
            if overlay.control_center.hovered == inside && (!inside || overlay.control_center.was_hovered) {
                return;
            }
        }
        let mut overlay = Self::write();

        overlay.control_center.hovered = inside;

        if inside {
            overlay.control_center.was_hovered = true;
        }

        if !inside && overlay.control_center.was_hovered {
            overlay.control_center.hide();
        }
    }

    pub fn hover_power_menu(inside: bool) {
        {
            let overlay = Self::read();
            if overlay.power_menu.hovered == inside && (!inside || overlay.power_menu.was_hovered) {
                return;
            }
        }
        let mut overlay = Self::write();

        overlay.power_menu.hovered = inside;

        if inside {
            overlay.power_menu.was_hovered = true;
        }

        if !inside && overlay.power_menu.was_hovered {
            overlay.power_menu.hide();
        }
    }

    pub fn hover_utility(inside: bool) {
        {
            let overlay = Self::read();
            if overlay.utility.hovered == inside && (!inside || overlay.utility.was_hovered) {
                return;
            }
        }
        let mut overlay = Self::write();

        overlay.utility.hovered = inside;

        if inside {
            overlay.utility.was_hovered = true;
        }

        if !inside && overlay.utility.was_hovered {
            overlay.utility.hide();
        }
    }

    pub fn hover_popup(id: u32, inside: bool) {
        {
            let overlay = Self::read();
            if inside && overlay.hovered_popup == Some(id) {
                return;
            }
            if !inside && overlay.hovered_popup != Some(id) {
                return;
            }
        }
        let mut overlay = Self::write();

        if inside {
            overlay.hovered_popup = Some(id);
        } else if overlay.hovered_popup == Some(id) {
            overlay.hovered_popup = None;
        }
    }

    // only the popup goes, the notification stays in the list
    pub fn close_popup(id: u32) {
        let listed: Vec<u32> = Notifications::read().list().iter().map(Notification::id).collect();

        let mut overlay = Self::write();

        // ids of notifications already gone are dropped, so the list stays short
        overlay.closed_popups.retain(|closed| listed.contains(closed));

        overlay.closed_popups.push(id);
    }

    pub fn hover_bar(inside: bool) {
        let mut overlay = Self::write();

        overlay.bar_hovered = inside;

        overlay.dismiss_if_left();
    }

    /*
     * a panel closes once the pointer has been on it and then left it and
     * the bar; not while a password is being typed, it would be lost
     */
    fn dismiss_if_left(&mut self) {
        let bar_hovered = self.bar_hovered;

        let typing = self.password_for.is_some();

        let mut panels = vec![
            &mut self.power_menu,
            &mut self.app_menu,
            &mut self.settings_menu,
            &mut self.control_center,
            &mut self.sysmon,
            &mut self.power_devices,
            &mut self.audio_control,
            &mut self.home_assistant,
        ];

        if !typing {
            panels.push(&mut self.utility);
        }

        for panel in panels {
            let left = panel.was_hovered && !panel.hovered && !bar_hovered;

            if panel.shown && left {
                panel.hide();
            }
        }
    }
}
