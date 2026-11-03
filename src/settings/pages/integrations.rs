use amane::Service;

use super::super::page::Page;
use super::super::{Confirm, Settings, Shown, switch};

// the key, title, what the row says, and what turning it on does
const INTEGRATIONS: [(&str, &str, &str, &str); 12] = [
    (
        "integration_gtk",
        "GTK",
        "GTK 3/4 and desktop preferences",
        "This writes generated GTK themes, changes gtk-theme, icon-theme and color-scheme through gsettings, replaces ~/.config/gtk-3.0/gtk.css and ~/.config/gtk-4.0/gtk.css with symlinks, and restarts xdg-desktop-portal-gnome.",
    ),
    (
        "integration_flux",
        "Flux File Manager",
        "Live GTK4/Libadwaita theme and MIME icon colors for Flux",
        "This writes generated Flux themes to ~/.local/share/flux/themes and ~/.config/flux/style.css, and updates ~/.config/flux/config.toml so running Flux windows live-reload colors automatically.",
    ),
    (
        "integration_strata",
        "Strata File Manager",
        "Live theme, syntax highlighting, and custom styling for Strata",
        "This writes generated Strata themes to ~/.config/strata/themes/amane.toml, custom CSS to ~/.config/strata/style.css, and updates ~/.config/strata/settings.toml so Strata windows live-reload colors automatically.",
    ),
    (
        "integration_terminal",
        "Terminals",
        "Alacritty, kitty and foot",
        "This overwrites the Alacritty, kitty and foot palette files, reloads kitty windows, signals running foot processes, and writes color escape sequences to their terminals.",
    ),
    (
        "integration_tmux",
        "tmux",
        "Status line and pane colors",
        "This overwrites tmux-colors.conf and asks running tmux servers to source that file.",
    ),
    (
        "integration_vesktop",
        "Vesktop",
        "Vencord Quick CSS",
        "This edits ~/.config/vesktop/settings/quickCss.css. Content outside the managed theme block is kept, but the file itself is rewritten.",
    ),
    (
        "integration_spotify",
        "Spotify",
        "Spicetify color stylesheet",
        "This creates and overwrites the Spotify stylesheet under your cache directory.",
    ),
    (
        "integration_btop",
        "btop",
        "Generated terminal monitor theme",
        "This creates and overwrites ~/.config/btop/themes/amane.theme.",
    ),
    (
        "integration_cava",
        "Cava",
        "Generated visualizer theme",
        "This creates and overwrites ~/.config/cava/themes/amane.",
    ),
    (
        "integration_cosmic",
        "COSMIC / Iced",
        "COSMIC apps and libcosmic iced toolkit",
        "This writes generated COSMIC themes to ~/.config/cosmic, updating cosmic-files, cosmic-edit, and all libcosmic iced applications.",
    ),
    (
        "integration_zed",
        "Zed Editor",
        "Theme for Zed editor",
        "This generates ~/.config/zed/themes/amane.json and updates ~/.config/zed/settings.json to use the dynamic theme.",
    ),
    (
        "integration_obsidian",
        "Obsidian",
        "Theme snippet for Obsidian vaults",
        "This writes generated dynamic amane CSS snippets into all configured Obsidian vaults and keeps them synchronized with the desktop wallpaper colors.",
    ),
];

// turning one on asks first, turning it off doesn't
pub fn build(page: &mut Page) {
    for (key, title, detail, warning) in INTEGRATIONS {
        switch::add_with(page, key, title, detail, move || {
            if Settings::read().staged_flag(key) {
                Settings::stage(key, false);

                return;
            }

            Shown::write().confirm = Some(Confirm::Integration { key, title, warning });
        });
    }

    page.end_group();
}
