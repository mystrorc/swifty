# Swifty Shell

<div align="center">
  <h1>Swifty</h1>
  <p><strong>A modern, opinionated Wayland desktop shell written in Rust with Amane, built for Niri.</strong></p>

  <p>
    <img alt="amane 0.1" src="https://img.shields.io/badge/AMANE-0.1-89b4fa?style=flat-square&labelColor=181825">
    <img alt="Niri" src="https://img.shields.io/badge/COMPOSITOR-NIRI-89b4fa?style=flat-square&labelColor=181825">
    <img alt="Rust" src="https://img.shields.io/badge/LANGUAGE-RUST-89b4fa?style=flat-square&labelColor=181825">
  </p>
</div>

## Features & Highlights

Swifty ([`mystrorc/swifty`](https://github.com/mystrorc/swifty)) is an opinionated, highly refined desktop shell built with Rust and Amane for the Niri Wayland compositor. While originally based on Suzuha, Swifty introduces **over 20,000 lines of brand-new subsystems, native hardware managers, and cross-application integrations** that do not exist in upstream:

---

### 1. New Subsystems & Hardware Management (Built from Scratch)

- **Native Home Assistant Smart Home Integration (`src/bar/home_assistant.rs`, `src/overlay/home_assistant.rs`):**
  - **Conditional Bar Pill (`󰌵`):** Positioned immediately to the right of the battery pill; dynamically appears only when connected to the local home network and Home Assistant is reachable. Features an illuminated bulb and dynamic badge indicating active lights.
  - **Quick Action:** Right-clicking the pill turns off all lights instantly without opening any menu.
  - **Liquid Popover Flyout:** Left-clicking opens a live smart home control panel with optimistic state switches for discovered light and switch entities, plus an "All Off" master toggle.
  - **Secure Storage:** Token and instance URL stored in `~/.config/amane/homeassistant.json`.

- **Elephant Search, Calculation & Window Engine (`src/overlay/launcher/elephant.rs`):**
  - Native Unix-socket client connecting the launcher directly to the **Elephant** Go daemon (`$XDG_RUNTIME_DIR/elephant/elephant.sock`).
  - **Inline Math / Calculator (`calc` provider):** Type mathematical equations (e.g. `24 * 16`, `sqrt(144)`, `15% of 80`) and receive instant evaluated results with copy-to-clipboard on Enter.
  - **Window Switcher (`windows` provider):** Search and jump straight to open windows across all Niri workspaces.
  - **Clipboard History (`clipboard` provider):** Search and paste from clipboard history directly via `c ` or `clip:`.
  - **Web Search Integration (`websearch` provider):** Quick web searches triggered by `?`, `w `, or `web:`.

- **Adaptive Floating Desktop Widgets (`src/floating/windows.rs`):**
  - Background desktop widgets (calendar, clock, weather) listen live to Niri window geometry. When an application is tiled or snapped half-screen, the widgets automatically slide to the unoccupied side of the screen so information is never covered.

- **10-Band Graphic Audio Equalizer & DSP Engine (`src/overlay/control_center/audio_eq.rs`):**
  - Native PipeWire parametric/graphic DSP engine integrated directly into the Control Center.
  - Real-time `[-15.0 dB, +15.0 dB]` frequency faders with live audio filter nodes.
  - Curated hardware presets specifically tuned for the **Sony WH-1000XM3** and **Razer Kraken TE**.
  - Mixer shortcut invokes `wiremix` instead of heavy legacy `pavucontrol`.

- **Native Graphical Display & Monitor Layout Manager (`src/settings/pages/monitors.rs`, `src/monitors.rs`):**
  - Replaces external shell scripts with a native visual canvas to arrange, scale, rotate, and set refresh rates for displays.
  - **Undocked Safety Guard:** Automatically detects when the laptop is disconnected from a dock or boots without an external display, guaranteeing `eDP-1` is powered ON at scale `2.0` (`3200x1800@59.982`), preventing black screen lockouts.
  - **Docked Consistency Guard:** Matches external displays by their hardware EDID identity (Make/Model/Serial) rather than transient connector names (`DP-5` vs `DP-1`), preventing USB-C/Thunderbolt dock re-indexing from resetting your scale or layout.
  - **Clamshell Mode:** Powers off laptop display when lid is closed while docked, and restores it when opened.
  - **Dynamic Window Rescue:** Open windows automatically migrate to remaining active monitors when a display is disconnected or disabled.

- **Monitor Backlight & Brightness Subsystem:**
  - Integrated slider controls and IPC shims supporting both laptop internal backlight (`brightnessctl`) and external desktop monitors via DDC/CI.

- **Shake-to-Find Cursor (`src/shake_cursor.rs`):**
  - macOS-style cursor shake-to-grow detection with custom sensitivity and dampening physics so you never lose your pointer on high-resolution displays.

- **Power Devices & Battery Health (`src/overlay/power_devices.rs`):**
  - Live UPower integration tracking battery charge status, discharge rates, connected Bluetooth peripheral levels, and system power profiles (Power Saver, Balanced, Performance).

- **Kanata Keyboard Remapper & Input Device Settings (`src/settings/pages/kanata.rs`, `src/settings/pages/input.rs`):**
  - Graphical configuration interface for Kanata keyboard layers.
  - Per-device pointer controls: independent speed sliders, flat acceleration toggles, natural scrolling, and sensor DPI switching for trackballs and mice.

- **Default Applications Manager (`src/settings/pages/default_apps.rs`):**
  - Graphical XDG MIME handler configuration for default web browsers, file managers, terminal emulators, and text editors.

- **Launcher Key Promoter (`src/overlay/launcher/shortcuts.rs`):**
  - Actively parses your Niri keybindings and displays the exact keyboard shortcut chord (using `Super+...`) alongside app search results, teaching you shortcuts as you work.

---

### 2. Live Dynamic Ecosystem Theming Bridges (`src/integrations/`)

Swifty acts as a dynamic theming hub for your entire desktop. Changing a wallpaper or color scheme generates and hot-reloads matching themes across third-party apps:

- **Flux File Manager (`src/integrations/flux.rs`, `flux.css`):**
  - Over 1,600 lines of custom CSS injecting frosted-glass transparency, matching accent borders, and dynamic color tokens.
- **Strata (`src/integrations/strata.rs`):**
  - Real-time synchronization of system color schemes and corner rounding into Strata.
- **COSMIC & Iced Applications (`src/integrations/cosmic.rs`):**
  - Embedded native D-Bus `com.system76.CosmicSettingsDaemon` service bridge broadcasting live theme changes to running COSMIC apps (`cosmic-files`, `cosmic-edit`), with 85% background glass transparency.
- **Zed Editor (`src/integrations/zed.rs`):**
  - Generates custom matching JSON color schemes at `~/.config/zed/themes/amane.json`.
- **Terminals (Alacritty & Ghostty) (`src/integrations/terminal.rs`):**
  - Generates palette definitions in `~/.config/alacritty/amane-colors.toml` and Ghostty configs with instant live reloading.
- **GTK 3 & GTK 4 (`src/integrations/gtk.rs`):**
  - Automatically updates GTK CSS symlinks, color-scheme preferences, and accent tokens.

---

### 3. Top Bar, Layout & UI Refinements

- **Flush Top Alignment (`y = 0` Edge Alignment):**
  - Top bar spacing has been eliminated, allowing capsules and pills to sit flush against the top screen edge with a clean 32px height.
- **Top Bar Quick Settings Pill & Pulldown (`src/overlay/settings_menu.rs`, `src/bar/workspaces.rs`):**
  - Dedicated Settings icon capsule (`󰒓`) on the top-left next to the distro logo.
  - Drops down an animated liquid preferences menu with category subtabs (Appearance, Bar, Launcher, Input, Integrations) and animation speed sliders (0.5× / 1.0× / 1.5× / 2.0×) for instant tweaks without launching the full Settings window.
- **Dynamic Network Tray Icon & Spaced System Tray (`src/bar/system.rs`):**
  - Automatically switches between an Ethernet LAN icon (`󰈀`) when wired and Wi-Fi (``) when wireless.
  - Balanced 18px icon bounding slots with a 10px visual gap to prevent glyph crowding.
- **Redesigned Calendar & 7-Day Weather Pulldown (`src/overlay/control_center/calendar.rs`):**
  - 820px dropdown featuring 7-day Open-Meteo forecasts, large digital clock, monthly calendar grid, and persistent agenda reminders (`~/.local/state/amane/reminders.json`).
- **Unified Media, Audio & Equalizer Tab (`src/overlay/control_center/media.rs`):**
  - Combines Now Playing, album art, live Cava visualizer, waveform seeking, volume faders, sink picker, and the 10-band EQ into a single screen.
- **System Info & Hardware Overview (`src/settings/pages/user.rs`):**
  - Hero System Card displaying distro logo, kernel release, uptime, package counts, CPU specs, RAM usage, and GPU info (`lspci`).
- **Settings Tab Streamlining:**
  - Removed redundant tabs (*About*, *System Info*, *Behavior*, *Plugins*).
  - Moved Animation Speed and Wallpaper transitions directly into the Appearance tab and Quick Settings pulldown.

---

## Installation & Running

Clone the repository:

```sh
git clone https://github.com/mystrorc/swifty.git ~/git/Swifty
```

Build the shell:

```sh
cargo build --release
```

Or start it with Niri in `~/.config/niri/config.kdl`:

```kdl
spawn-at-startup "swifty"
```
