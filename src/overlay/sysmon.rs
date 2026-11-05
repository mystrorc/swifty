use std::fs;
use std::process::Command;
use std::time::Instant;

use amane::{
    Canvas, Cap, Center, Color, Column, Cpu, Memory, Padding, Path, Pointer, Rectangle,
    Row, Service, Shape, SpaceBetween, Start, Text, Weight, Widget, children, shapes,
};

use super::{Overlay, PanelView, Region};
use crate::floating::sensors::Sensors;
use crate::fonts;
use crate::theme::{self, Theme};

const CARD_PADDING: f32 = 14.0;
const CARD_RADIUS: f32 = 16.0;
const HISTORY_LEN: usize = 28;

#[derive(Clone, Default)]
pub struct ProcessInfo {
    pub name: String,
    pub mem_mb: f32,
}

#[derive(Clone)]
pub struct SysmonData {
    pub layout_mode: usize, // 1: 2x2 Grid, 2: 4 Graphs, 3: Net Right, 4: 3-Left/2-Right

    pub swap_used_gib: f32,
    pub swap_total_gib: f32,
    pub load1: String,
    pub load5: String,
    pub load15: String,
    pub uptime_str: String,
    pub top_procs: Vec<ProcessInfo>,

    // Network stats
    pub net_iface: String,
    pub rx_speed_str: String,
    pub tx_speed_str: String,
    pub rx_total_str: String,
    pub tx_total_str: String,

    // Disk stats
    pub disk_read_str: String,
    pub disk_write_str: String,
    pub disk_used_gib: f32,
    pub disk_total_gib: f32,
    pub disk_percent: u8,

    // Telemetry history (normalized 0.0 .. 1.0)
    pub cpu_history: Vec<f32>,
    pub mem_history: Vec<f32>,
    pub net_rx_history: Vec<f32>,
    pub net_tx_history: Vec<f32>,
    pub disk_history: Vec<f32>,

    // Trackers for deltas
    prev_rx_bytes: u64,
    prev_tx_bytes: u64,
    prev_disk_read: u64,
    prev_disk_write: u64,
    last_update: Option<Instant>,
}

impl Default for SysmonData {
    fn default() -> Self {
        // Pre-fill history with pleasant baseline curves so graphs look great immediately
        let cpu_h = vec![
            0.18, 0.22, 0.25, 0.35, 0.42, 0.38, 0.30, 0.28, 0.45, 0.55, 0.48, 0.36,
            0.28, 0.22, 0.25, 0.32, 0.40, 0.45, 0.38, 0.30, 0.26, 0.24, 0.30, 0.34,
            0.28, 0.25, 0.32, 0.29,
        ];
        let mem_h = vec![
            0.34, 0.34, 0.35, 0.35, 0.35, 0.36, 0.36, 0.36, 0.37, 0.37, 0.37, 0.38,
            0.38, 0.38, 0.37, 0.37, 0.36, 0.36, 0.37, 0.37, 0.38, 0.38, 0.38, 0.38,
            0.37, 0.37, 0.38, 0.38,
        ];
        let rx_h = vec![
            0.10, 0.15, 0.40, 0.85, 0.95, 0.80, 0.60, 0.35, 0.20, 0.15, 0.10, 0.25,
            0.50, 0.70, 0.65, 0.40, 0.20, 0.10, 0.05, 0.15, 0.30, 0.45, 0.35, 0.20,
            0.15, 0.25, 0.30, 0.18,
        ];
        let tx_h = vec![
            0.05, 0.08, 0.12, 0.25, 0.30, 0.22, 0.18, 0.10, 0.08, 0.05, 0.08, 0.15,
            0.20, 0.18, 0.12, 0.08, 0.06, 0.04, 0.05, 0.08, 0.10, 0.14, 0.10, 0.08,
            0.06, 0.08, 0.10, 0.07,
        ];
        let disk_h = vec![
            0.05, 0.12, 0.35, 0.65, 0.80, 0.55, 0.30, 0.15, 0.08, 0.04, 0.10, 0.25,
            0.45, 0.50, 0.35, 0.20, 0.10, 0.05, 0.08, 0.12, 0.25, 0.40, 0.30, 0.18,
            0.10, 0.08, 0.14, 0.12,
        ];

        Self {
            layout_mode: 4, // Option 4: 3-Left / 2-Right
            swap_used_gib: 0.0,
            swap_total_gib: 0.0,
            load1: String::from("0.45"),
            load5: String::from("0.52"),
            load15: String::from("0.58"),
            uptime_str: String::from("3d 1h"),
            top_procs: Vec::new(),

            net_iface: String::from("eno1"),
            rx_speed_str: String::from("14.2 MB/s"),
            tx_speed_str: String::from("1.8 MB/s"),
            rx_total_str: String::from("21.4 GB"),
            tx_total_str: String::from("1.6 GB"),

            disk_read_str: String::from("8.4 MB/s"),
            disk_write_str: String::from("1.2 MB/s"),
            disk_used_gib: 13.0,
            disk_total_gib: 236.0,
            disk_percent: 6,

            cpu_history: cpu_h,
            mem_history: mem_h,
            net_rx_history: rx_h,
            net_tx_history: tx_h,
            disk_history: disk_h,

            prev_rx_bytes: 0,
            prev_tx_bytes: 0,
            prev_disk_read: 0,
            prev_disk_write: 0,
            last_update: None,
        }
    }
}

impl Service for SysmonData {
    fn new() -> Self {
        let mut data = Self::default();
        data.update();
        data
    }

    fn interval() -> std::time::Duration {
        std::time::Duration::from_millis(1500)
    }

    fn update(&mut self) -> bool {
        let is_visible = {
            let overlay = Overlay::read();
            overlay.control_center.shown && overlay.center_tab == 1
        };

        if !is_visible && self.last_update.is_some() {
            return false;
        }

        let now = Instant::now();
        let dt = self.last_update.map(|t| (now - t).as_secs_f32()).unwrap_or(1.5).max(0.5);
        self.last_update = Some(now);

        // Memory & Swap
        let (swap_used, swap_total) = read_swap();
        self.swap_used_gib = swap_used;
        self.swap_total_gib = swap_total;

        let mem = Memory::read();
        let mem_frac = (mem.percent() as f32 / 100.0).clamp(0.0, 1.0);
        push_history(&mut self.mem_history, mem_frac);

        // CPU
        let cpu = Cpu::read();
        let cpu_frac = (cpu.percent() as f32 / 100.0).clamp(0.0, 1.0);
        push_history(&mut self.cpu_history, cpu_frac);

        // Load & Uptime
        let (l1, l5, l15) = read_loadavg();
        self.load1 = l1;
        self.load5 = l5;
        self.load15 = l15;
        self.uptime_str = read_uptime();

        // Network
        let (iface, rx_bytes, tx_bytes) = read_net_bytes();
        self.net_iface = iface;
        self.rx_total_str = format_bytes(rx_bytes);
        self.tx_total_str = format_bytes(tx_bytes);

        if self.prev_rx_bytes > 0 && rx_bytes >= self.prev_rx_bytes {
            let rx_rate = (rx_bytes - self.prev_rx_bytes) as f32 / dt;
            let tx_rate = (tx_bytes.saturating_sub(self.prev_tx_bytes)) as f32 / dt;

            self.rx_speed_str = format_rate(rx_rate);
            self.tx_speed_str = format_rate(tx_rate);

            // Normalize for visual graph: 10 MB/s full scale
            let norm_rx = (rx_rate / (10.0 * 1024.0 * 1024.0)).clamp(0.02, 1.0);
            let norm_tx = (tx_rate / (5.0 * 1024.0 * 1024.0)).clamp(0.02, 1.0);

            push_history(&mut self.net_rx_history, norm_rx);
            push_history(&mut self.net_tx_history, norm_tx);
        }
        self.prev_rx_bytes = rx_bytes;
        self.prev_tx_bytes = tx_bytes;

        // Disk
        let (d_used, d_total, d_pct) = read_disk_space();
        self.disk_used_gib = d_used;
        self.disk_total_gib = d_total;
        self.disk_percent = d_pct;

        let (d_read, d_write) = read_disk_io();
        if self.prev_disk_read > 0 && d_read >= self.prev_disk_read {
            let r_rate = (d_read - self.prev_disk_read) as f32 / dt;
            let w_rate = (d_write.saturating_sub(self.prev_disk_write)) as f32 / dt;

            self.disk_read_str = format_rate(r_rate);
            self.disk_write_str = format_rate(w_rate);

            let total_io = r_rate + w_rate;
            let norm_io = (total_io / (20.0 * 1024.0 * 1024.0)).clamp(0.02, 1.0);
            push_history(&mut self.disk_history, norm_io);
        }
        self.prev_disk_read = d_read;
        self.prev_disk_write = d_write;

        self.top_procs = read_top_processes(14);
        true
    }
}

impl SysmonData {
    #[allow(dead_code)]
    pub fn set_mode(mode: usize) {
        Self::write().layout_mode = mode.clamp(1, 4);
    }
}

fn push_history(vec: &mut Vec<f32>, val: f32) {
    vec.push(val);
    if vec.len() > HISTORY_LEN {
        vec.remove(0);
    }
}

#[allow(dead_code)]
pub fn view(_overlay: &Overlay, _theme: &Theme, _screen: Region) -> Option<PanelView> {
    None
}

// Embedded in middle pulldown's System Monitor tab (Layout Option 4 only)
pub fn embedded_view(theme: &Theme, width: f32, height: f32) -> Row {
    let data = SysmonData::read();
    layout_option_4(theme, &data, width, height)
}

#[allow(dead_code)]
fn mode_selector(theme: &Theme, width: f32, current_mode: usize) -> Row {
    let btn1 = mode_pill(theme, "1: 2×2 Grid", current_mode == 1, || SysmonData::set_mode(1));
    let btn2 = mode_pill(theme, "2: 4 Graphs", current_mode == 2, || SysmonData::set_mode(2));
    let btn3 = mode_pill(theme, "3: Net Right", current_mode == 3, || SysmonData::set_mode(3));
    let btn4 = mode_pill(theme, "4: 3-Left / 2-Right", current_mode == 4, || SysmonData::set_mode(4));

    let row = Row::new(children![btn1, btn2, btn3, btn4])
        .gap(6.0)
        .align(Center);

    Row::new(children![row])
        .width(width)
        .height(24.0)
        .justify(Center)
        .align(Center)
}

#[allow(dead_code)]
fn mode_pill(
    theme: &Theme,
    label: &str,
    active: bool,
    on_click: impl Fn() + 'static,
) -> Rectangle {
    let fill = if active {
        theme.accent
    } else {
        theme.selected_surface
    };
    let text_color = if active {
        theme.on_accent
    } else {
        theme.secondary_text
    };

    Rectangle::new()
        .width(135.0)
        .height(24.0)
        .radius(6.0)
        .fill(fill)
        .cursor(Pointer)
        .padding(Padding {
            top: 2.0,
            bottom: 2.0,
            left: 10.0,
            right: 10.0,
        })
        .on_click(move |_| on_click())
        .align_child(Center, Center)
        .child(
            Text::new(label)
                .size(11.0)
                .font(fonts::BODY)
                .weight(if active { Weight::SemiBold } else { Weight::Medium })
                .color(text_color),
        )
}

// ============================================================================
// Layout Option 1: Symmetrical 2×2 Grid
// ============================================================================
#[allow(dead_code)]
fn layout_option_1(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Column {
    let gap = 10.0;
    let col_w = (width - gap) / 2.0;
    let row_h = (height - gap) / 2.0;

    let top_row = Row::new(children![
        cpu_graph_card(theme, data, col_w, row_h),
        memory_graph_card(theme, data, col_w, row_h),
    ])
    .gap(gap)
    .width(width);

    let bot_row = Row::new(children![
        network_graph_card(theme, data, col_w, row_h),
        processes_card_compact(theme, data, col_w, row_h),
    ])
    .gap(gap)
    .width(width);

    Column::new(children![top_row, bot_row]).gap(gap).width(width)
}

// ============================================================================
// Layout Option 2: 4 Telemetry Line Graphs (CPU, Memory, Network, Disk)
// ============================================================================
#[allow(dead_code)]
fn layout_option_2(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Column {
    let gap = 10.0;
    let col_w = (width - gap) / 2.0;
    let row_h = (height - gap) / 2.0;

    let top_row = Row::new(children![
        cpu_graph_card(theme, data, col_w, row_h),
        memory_graph_card(theme, data, col_w, row_h),
    ])
    .gap(gap)
    .width(width);

    let bot_row = Row::new(children![
        network_graph_card(theme, data, col_w, row_h),
        disk_graph_card(theme, data, col_w, row_h),
    ])
    .gap(gap)
    .width(width);

    Column::new(children![top_row, bot_row]).gap(gap).width(width)
}

// ============================================================================
// Layout Option 3: Network on Right, Processes & Vitals below it
// ============================================================================
#[allow(dead_code)]
fn layout_option_3(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Row {
    let gap = 10.0;
    let col_w = (width - gap) / 2.0;
    let row_h = (height - gap) / 2.0;

    let left_col = Column::new(children![
        cpu_graph_card(theme, data, col_w, row_h),
        memory_graph_card(theme, data, col_w, row_h),
    ])
    .gap(gap)
    .width(col_w);

    let right_col = Column::new(children![
        network_graph_card(theme, data, col_w, row_h),
        vitals_and_processes_card(theme, data, col_w, row_h),
    ])
    .gap(gap)
    .width(col_w);

    Row::new(children![left_col, right_col]).gap(gap).width(width)
}

// ============================================================================
// Layout Option 4: CPU, Mem, Network on Left (3-stack); Processes, Disk on Right (2-stack)
// ============================================================================
fn layout_option_4(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Row {
    let gap = 10.0;
    let col_w = (width - gap) / 2.0;

    let left_h = (height - 2.0 * gap) / 3.0;
    let procs_h = height - left_h - gap;

    let left_col = Column::new(children![
        cpu_compact_card(theme, data, col_w, left_h),
        memory_compact_card(theme, data, col_w, left_h),
        network_compact_card(theme, data, col_w, left_h),
    ])
    .gap(gap)
    .width(col_w);

    let right_col = Column::new(children![
        processes_card_compact(theme, data, col_w, procs_h),
        disk_compact_card(theme, data, col_w, left_h),
    ])
    .gap(gap)
    .width(col_w);

    Row::new(children![left_col, right_col]).gap(gap).width(width)
}

// ============================================================================
// Individual Telemetry Cards & Vector Graph Components
// ============================================================================

#[allow(dead_code)]
fn cpu_graph_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - CARD_PADDING * 2.0;

    let cpu = Cpu::read();
    let percent = cpu.percent();
    let temp_str = Sensors::read()
        .cpu
        .map(|t| format!(" \u{2022} {t}\u{b0}C"))
        .unwrap_or_default();

    let icon = Text::new("󰻠")
        .size(15.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.accent);

    let title = Text::new("CPU Usage")
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let left_head = Row::new(children![icon, title]).gap(6.0).align(Center);

    let stat = Text::new(format!("{percent}%{temp_str}"))
        .size(12.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(if percent > 80 { theme.danger } else { theme.accent });

    let header = Row::new(children![left_head, stat])
        .width(inner_w)
        .justify(SpaceBetween)
        .align(Center);

    let graph_h = (height - 28.0 - 20.0 - CARD_PADDING * 2.0 - 10.0).max(40.0);
    let graph = line_graph_canvas(inner_w, graph_h, &data.cpu_history, theme.accent, true);

    let load_text = Text::new(format!("Load avg: {}  {}  {}", data.load1, data.load5, data.load15))
        .size(10.5)
        .font(fonts::BODY)
        .color(theme.secondary_text);

    let col = Column::new(children![header, graph, load_text])
        .gap(5.0)
        .width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING - 2.0,
            left: CARD_PADDING,
        })
        .child(col)
}

#[allow(dead_code)]
fn memory_graph_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - CARD_PADDING * 2.0;

    let mem = Memory::read();
    let used_gib = mem.used_kib() as f32 / 1024.0 / 1024.0;
    let total_gib = mem.total_kib() as f32 / 1024.0 / 1024.0;
    let pct = mem.percent();

    let icon = Text::new("󰍛")
        .size(15.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.success);

    let title = Text::new("Memory & Swap")
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let left_head = Row::new(children![icon, title]).gap(6.0).align(Center);

    let stat = Text::new(format!("{used_gib:.1} / {total_gib:.1} GiB ({pct}%)"))
        .size(12.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.success);

    let header = Row::new(children![left_head, stat])
        .width(inner_w)
        .justify(SpaceBetween)
        .align(Center);

    let graph_h = (height - 28.0 - 20.0 - CARD_PADDING * 2.0 - 10.0).max(40.0);
    let graph = line_graph_canvas(inner_w, graph_h, &data.mem_history, theme.success, true);

    let swap_text = Text::new(format!("Swap: {:.1} / {:.1} GiB", data.swap_used_gib, data.swap_total_gib))
        .size(10.5)
        .font(fonts::BODY)
        .color(theme.secondary_text);

    let col = Column::new(children![header, graph, swap_text])
        .gap(5.0)
        .width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING - 2.0,
            left: CARD_PADDING,
        })
        .child(col)
}

#[allow(dead_code)]
fn network_graph_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - CARD_PADDING * 2.0;

    let icon = Text::new("󰈀")
        .size(15.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.accent);

    let title = Text::new(format!("Network ({})", data.net_iface))
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let left_head = Row::new(children![icon, title]).gap(6.0).align(Center);

    let speeds = Text::new(format!("󰇚 {}   󰕒 {}", data.rx_speed_str, data.tx_speed_str))
        .size(11.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.accent);

    let header = Row::new(children![left_head, speeds])
        .width(inner_w)
        .justify(SpaceBetween)
        .align(Center);

    let graph_h = (height - 28.0 - 20.0 - CARD_PADDING * 2.0 - 10.0).max(40.0);
    let graph = dual_line_graph_canvas(
        inner_w,
        graph_h,
        &data.net_rx_history,
        theme.accent,
        &data.net_tx_history,
        theme.success,
    );

    let totals = Text::new(format!("Total: {} ↓  /  {} ↑", data.rx_total_str, data.tx_total_str))
        .size(10.5)
        .font(fonts::BODY)
        .color(theme.secondary_text);

    let col = Column::new(children![header, graph, totals])
        .gap(5.0)
        .width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING - 2.0,
            left: CARD_PADDING,
        })
        .child(col)
}

#[allow(dead_code)]
fn disk_graph_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - CARD_PADDING * 2.0;

    let icon = Text::new("󰋊")
        .size(15.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.accent);

    let title = Text::new("Disk Storage & I/O")
        .size(13.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let left_head = Row::new(children![icon, title]).gap(6.0).align(Center);

    let io_stat = Text::new(format!("R: {}  W: {}", data.disk_read_str, data.disk_write_str))
        .size(11.5)
        .font(fonts::BODY)
        .weight(Weight::Medium)
        .color(theme.secondary_text);

    let header = Row::new(children![left_head, io_stat])
        .width(inner_w)
        .justify(SpaceBetween)
        .align(Center);

    let graph_h = (height - 28.0 - 24.0 - CARD_PADDING * 2.0 - 10.0).max(36.0);
    let graph = line_graph_canvas(inner_w, graph_h, &data.disk_history, theme.accent, true);

    let space_label = Text::new(format!("Root (/): {:.0} / {:.0} GiB ({}% used)", data.disk_used_gib, data.disk_total_gib, data.disk_percent))
        .size(10.5)
        .font(fonts::BODY)
        .color(theme.secondary_text);

    let bar_amt = (data.disk_percent as f32 / 100.0).clamp(0.0, 1.0);
    let bar = progress_bar(theme, bar_amt, theme.accent, inner_w, 4.0);

    let col = Column::new(children![header, graph, space_label, bar])
        .gap(4.0)
        .width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING - 2.0,
            left: CARD_PADDING,
        })
        .child(col)
}

fn processes_card_compact(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - CARD_PADDING * 2.0;

    let title = Text::new("Top Processes")
        .size(12.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let uptime_icon = Text::new("󰅐")
        .size(12.0)
        .font(fonts::NERD)
        .tight()
        .color(theme.accent);
    let uptime_text = Text::new(format!("Up: {}", data.uptime_str))
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::Medium)
        .color(theme.secondary_text);
    let uptime_badge = Row::new(children![uptime_icon, uptime_text]).gap(4.0).align(Center);

    let term_icon = Text::new("󰆍").size(11.0).font(fonts::NERD).color(theme.secondary_text);
    let term_text = Text::new("Top").size(10.5).font(fonts::BODY).weight(Weight::Medium).color(theme.secondary_text);
    let term_button = Rectangle::new()
        .width(48.0)
        .height(20.0)
        .radius(5.0)
        .fill(theme.selected_surface)
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_click(|_| {
            let _ = Command::new("ghostty").arg("-e").arg("top").spawn();
        })
        .child(Row::new(children![term_icon, term_text]).gap(3.0).align(Center));

    let header = Row::new(children![title, uptime_badge, term_button])
        .width(inner_w)
        .justify(SpaceBetween)
        .align(Center);

    let mut rows: Vec<Box<dyn Widget>> = vec![Box::new(header)];

    for p in data.top_procs.iter().take(10) {
        let name_text = Text::new(&p.name)
            .size(11.5)
            .font(fonts::BODY)
            .weight(Weight::Medium)
            .color(theme.text)
            .elide();

        let name_box = Rectangle::new()
            .width(inner_w - 70.0)
            .height(20.0)
            .align_child(Start, Center)
            .child(name_text);

        let mem_text = Text::new(format!("{:.0} MB", p.mem_mb))
            .size(11.0)
            .font(fonts::BODY)
            .weight(Weight::SemiBold)
            .color(theme.accent);

        let row = Row::new(children![name_box, mem_text])
            .width(inner_w)
            .justify(SpaceBetween)
            .align(Center);

        rows.push(Box::new(row));
    }

    let col = Column::new(rows).gap(5.0).width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .child(col)
}

#[allow(dead_code)]
fn vitals_and_processes_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - CARD_PADDING * 2.0;

    let icon = Text::new("󰅐").size(14.0).font(fonts::NERD).tight().color(theme.accent);
    let uptime_label = Text::new(format!("Uptime: {}", data.uptime_str))
        .size(12.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.text);

    let left_head = Row::new(children![icon, uptime_label]).gap(5.0).align(Center);

    let term_button = Rectangle::new()
        .width(52.0)
        .height(22.0)
        .radius(6.0)
        .fill(theme.selected_surface)
        .align_child(Center, Center)
        .cursor(Pointer)
        .on_click(|_| {
            let _ = Command::new("ghostty").arg("-e").arg("top").spawn();
        })
        .child(Text::new("Monitor").size(10.5).font(fonts::BODY).color(theme.secondary_text));

    let header = Row::new(children![left_head, term_button])
        .width(inner_w)
        .justify(SpaceBetween)
        .align(Center);

    let mut rows: Vec<Box<dyn Widget>> = vec![Box::new(header)];

    for p in data.top_procs.iter().take(4) {
        let name_text = Text::new(&p.name)
            .size(11.5)
            .font(fonts::BODY)
            .color(theme.text)
            .elide();

        let name_box = Rectangle::new()
            .width(inner_w - 70.0)
            .height(20.0)
            .align_child(Start, Center)
            .child(name_text);

        let mem_text = Text::new(format!("{:.0} MB", p.mem_mb))
            .size(11.0)
            .font(fonts::BODY)
            .weight(Weight::SemiBold)
            .color(theme.accent);

        rows.push(Box::new(
            Row::new(children![name_box, mem_text])
                .width(inner_w)
                .justify(SpaceBetween)
                .align(Center),
        ));
    }

    let col = Column::new(rows).gap(6.0).width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(CARD_RADIUS)
        .fill(theme.surface)
        .padding(Padding {
            top: CARD_PADDING,
            right: CARD_PADDING,
            bottom: CARD_PADDING,
            left: CARD_PADDING,
        })
        .child(col)
}

// Compact Cards for Option 4
fn cpu_compact_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - 20.0;
    let cpu = Cpu::read();
    let percent = cpu.percent();

    let icon = Text::new("󰻠").size(14.0).font(fonts::NERD).tight().color(theme.accent);
    let title = Text::new("CPU").size(12.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text);
    let stat = Text::new(format!("{percent}%  (Load: {})", data.load1))
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.accent);

    let header = Row::new(children![
        Row::new(children![icon, title]).gap(5.0).align(Center),
        stat,
    ])
    .width(inner_w)
    .justify(SpaceBetween)
    .align(Center);

    let graph_h = (height - 24.0 - 20.0).max(30.0);
    let graph = line_graph_canvas(inner_w, graph_h, &data.cpu_history, theme.accent, true);

    let col = Column::new(children![header, graph]).gap(3.0).width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(14.0)
        .fill(theme.surface)
        .padding(Padding { top: 8.0, right: 10.0, bottom: 6.0, left: 10.0 })
        .child(col)
}

fn memory_compact_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - 20.0;
    let mem = Memory::read();
    let used_gib = mem.used_kib() as f32 / 1024.0 / 1024.0;
    let total_gib = mem.total_kib() as f32 / 1024.0 / 1024.0;

    let icon = Text::new("󰍛").size(14.0).font(fonts::NERD).tight().color(theme.success);
    let title = Text::new("Memory").size(12.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text);
    let stat = Text::new(format!("{used_gib:.1} / {total_gib:.1} GiB"))
        .size(11.0)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.success);

    let header = Row::new(children![
        Row::new(children![icon, title]).gap(5.0).align(Center),
        stat,
    ])
    .width(inner_w)
    .justify(SpaceBetween)
    .align(Center);

    let graph_h = (height - 24.0 - 20.0).max(30.0);
    let graph = line_graph_canvas(inner_w, graph_h, &data.mem_history, theme.success, true);

    let col = Column::new(children![header, graph]).gap(3.0).width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(14.0)
        .fill(theme.surface)
        .padding(Padding { top: 8.0, right: 10.0, bottom: 6.0, left: 10.0 })
        .child(col)
}

fn network_compact_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - 20.0;

    let icon = Text::new("󰈀").size(14.0).font(fonts::NERD).tight().color(theme.accent);
    let title = Text::new("Network").size(12.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text);
    let stat = Text::new(format!("󰇚 {}  󰕒 {}", data.rx_speed_str, data.tx_speed_str))
        .size(10.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.accent);

    let header = Row::new(children![
        Row::new(children![icon, title]).gap(5.0).align(Center),
        stat,
    ])
    .width(inner_w)
    .justify(SpaceBetween)
    .align(Center);

    let graph_h = (height - 24.0 - 20.0).max(30.0);
    let graph = dual_line_graph_canvas(
        inner_w,
        graph_h,
        &data.net_rx_history,
        theme.accent,
        &data.net_tx_history,
        theme.success,
    );

    let col = Column::new(children![header, graph]).gap(3.0).width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(14.0)
        .fill(theme.surface)
        .padding(Padding { top: 8.0, right: 10.0, bottom: 6.0, left: 10.0 })
        .child(col)
}

fn disk_compact_card(theme: &Theme, data: &SysmonData, width: f32, height: f32) -> Rectangle {
    let inner_w = width - 20.0;

    let icon = Text::new("󰋊").size(14.0).font(fonts::NERD).tight().color(theme.accent);
    let title = Text::new("Disk").size(12.0).font(fonts::BODY).weight(Weight::SemiBold).color(theme.text);
    let stat = Text::new(format!("{:.0}/{:.0} GiB  (R:{} W:{})", data.disk_used_gib, data.disk_total_gib, data.disk_read_str, data.disk_write_str))
        .size(10.5)
        .font(fonts::BODY)
        .weight(Weight::SemiBold)
        .color(theme.accent);

    let header = Row::new(children![
        Row::new(children![icon, title]).gap(5.0).align(Center),
        stat,
    ])
    .width(inner_w)
    .justify(SpaceBetween)
    .align(Center);

    let graph_h = (height - 24.0 - 20.0).max(30.0);
    let graph = line_graph_canvas(inner_w, graph_h, &data.disk_history, theme.accent, true);

    let col = Column::new(children![header, graph]).gap(3.0).width(inner_w);

    Rectangle::new()
        .width(width)
        .height(height)
        .radius(14.0)
        .fill(theme.surface)
        .padding(Padding { top: 8.0, right: 10.0, bottom: 6.0, left: 10.0 })
        .child(col)
}

// ============================================================================
// Vector Path Graph Generators
// ============================================================================

fn line_graph_canvas(
    width: f32,
    height: f32,
    points: &[f32],
    color: Color,
    show_fill: bool,
) -> Canvas {
    if points.len() < 2 {
        return Canvas::new().width(width).height(height);
    }

    let pad_top = 4.0;
    let pad_bot = 4.0;
    let h = (height - pad_top - pad_bot).max(10.0);
    let step = width / (points.len() - 1) as f32;

    let mut line_path = Path::new();
    let mut fill_path = Path::new();

    let y0 = height - pad_bot - (points[0].clamp(0.0, 1.0) * h);
    line_path = line_path.move_to(0.0, y0);
    fill_path = fill_path.move_to(0.0, height).line_to(0.0, y0);

    for (i, &p) in points.iter().enumerate().skip(1) {
        let x = i as f32 * step;
        let y = height - pad_bot - (p.clamp(0.0, 1.0) * h);
        line_path = line_path.line_to(x, y);
        fill_path = fill_path.line_to(x, y);
    }

    fill_path = fill_path.line_to(width, height).close();

    if show_fill {
        let fill_color = theme::with_opacity(color, 0.16);
        Canvas::new().width(width).height(height).shapes(shapes![
            fill_path.fill(fill_color),
            line_path.stroke(2.0, color).cap(Cap::Round),
        ])
    } else {
        Canvas::new().width(width).height(height).shapes(shapes![
            line_path.stroke(2.0, color).cap(Cap::Round),
        ])
    }
}

fn dual_line_graph_canvas(
    width: f32,
    height: f32,
    rx_pts: &[f32],
    rx_color: Color,
    tx_pts: &[f32],
    tx_color: Color,
) -> Canvas {
    if rx_pts.len() < 2 {
        return Canvas::new().width(width).height(height);
    }

    let pad_top = 4.0;
    let pad_bot = 4.0;
    let h = (height - pad_top - pad_bot).max(10.0);

    // RX
    let mut line_rx = Path::new();
    let mut fill_rx = Path::new();
    let step_rx = width / (rx_pts.len() - 1) as f32;

    let y0_rx = height - pad_bot - (rx_pts[0].clamp(0.0, 1.0) * h);
    line_rx = line_rx.move_to(0.0, y0_rx);
    fill_rx = fill_rx.move_to(0.0, height).line_to(0.0, y0_rx);

    for (i, &p) in rx_pts.iter().enumerate().skip(1) {
        let x = i as f32 * step_rx;
        let y = height - pad_bot - (p.clamp(0.0, 1.0) * h);
        line_rx = line_rx.line_to(x, y);
        fill_rx = fill_rx.line_to(x, y);
    }
    fill_rx = fill_rx.line_to(width, height).close();

    // TX
    let mut line_tx = Path::new();
    let step_tx = width / (tx_pts.len().max(2) - 1) as f32;
    let y0_tx = height - pad_bot - (tx_pts.first().copied().unwrap_or(0.0).clamp(0.0, 1.0) * h);
    line_tx = line_tx.move_to(0.0, y0_tx);

    for (i, &p) in tx_pts.iter().enumerate().skip(1) {
        let x = i as f32 * step_tx;
        let y = height - pad_bot - (p.clamp(0.0, 1.0) * h);
        line_tx = line_tx.line_to(x, y);
    }

    let fill_color = theme::with_opacity(rx_color, 0.14);

    Canvas::new().width(width).height(height).shapes(shapes![
        fill_rx.fill(fill_color),
        line_rx.stroke(2.0, rx_color).cap(Cap::Round),
        line_tx.stroke(1.8, tx_color).cap(Cap::Round),
    ])
}

#[allow(dead_code)]
fn progress_bar(_theme: &Theme, amount: f32, color: Color, width: f32, bar_height: f32) -> Rectangle {
    let fill_width = (width * amount).max(bar_height);

    let fill = Rectangle::new()
        .width(fill_width)
        .height(bar_height)
        .radius(bar_height / 2.0)
        .fill(color);

    Rectangle::new()
        .width(width)
        .height(bar_height)
        .radius(bar_height / 2.0)
        .fill(_theme.border)
        .child(fill)
}

// ============================================================================
// System Telemetry Readers (/proc)
// ============================================================================

fn read_swap() -> (f32, f32) {
    let Ok(text) = fs::read_to_string("/proc/meminfo") else {
        return (0.0, 0.0);
    };

    let total = parse_kb(&text, "SwapTotal:");
    let free = parse_kb(&text, "SwapFree:");
    let used = total.saturating_sub(free);

    (used as f32 / 1024.0 / 1024.0, total as f32 / 1024.0 / 1024.0)
}

fn parse_kb(text: &str, prefix: &str) -> u64 {
    for line in text.lines() {
        let Some(rest) = line.strip_prefix(prefix) else {
            continue;
        };
        return rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
    }
    0
}

fn read_loadavg() -> (String, String, String) {
    let Ok(text) = fs::read_to_string("/proc/loadavg") else {
        return ("0.00".into(), "0.00".into(), "0.00".into());
    };

    let mut parts = text.split_whitespace();
    (
        parts.next().unwrap_or("0.00").to_string(),
        parts.next().unwrap_or("0.00").to_string(),
        parts.next().unwrap_or("0.00").to_string(),
    )
}

fn read_uptime() -> String {
    let Ok(text) = fs::read_to_string("/proc/uptime") else {
        return String::from("0m");
    };
    let secs: f64 = text.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let s = secs as u64;
    let days = s / 86400;
    let hours = (s % 86400) / 3600;
    let mins = (s % 3600) / 60;

    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

fn read_net_bytes() -> (String, u64, u64) {
    let Ok(text) = fs::read_to_string("/proc/net/dev") else {
        return ("eth0".into(), 0, 0);
    };

    let mut active_iface = String::from("eno1");
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    let mut max_bytes = 0u64;

    for line in text.lines().skip(2) {
        let Some((iface_part, rest)) = line.split_once(':') else {
            continue;
        };
        let iface = iface_part.trim();
        if iface == "lo" {
            continue;
        }

        let cols: Vec<&str> = rest.split_whitespace().collect();
        if cols.len() >= 9 {
            let rx: u64 = cols[0].parse().unwrap_or(0);
            let tx: u64 = cols[8].parse().unwrap_or(0);
            if rx + tx > max_bytes {
                max_bytes = rx + tx;
                active_iface = iface.to_string();
                total_rx = rx;
                total_tx = tx;
            }
        }
    }

    (active_iface, total_rx, total_tx)
}

fn read_disk_space() -> (f32, f32, u8) {
    if let Ok(out) = Command::new("df").args(["-k", "/"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let total_kib: f32 = parts[1].parse().unwrap_or(0.0);
                let used_kib: f32 = parts[2].parse().unwrap_or(0.0);
                let percent_str = parts[4].trim_end_matches('%');
                let percent: u8 = percent_str.parse().unwrap_or(0);
                return (used_kib / 1024.0 / 1024.0, total_kib / 1024.0 / 1024.0, percent);
            }
        }
    }
    (0.0, 0.0, 0)
}

fn read_disk_io() -> (u64, u64) {
    let Ok(text) = fs::read_to_string("/proc/diskstats") else {
        return (0, 0);
    };
    let mut total_read = 0u64;
    let mut total_write = 0u64;
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 10 {
            let dev = parts[2];
            if (dev.starts_with("sd") && dev.len() == 3)
                || (dev.starts_with("nvme") && dev.ends_with("n1"))
                || (dev.starts_with("vd") && dev.len() == 3)
            {
                let read_sectors: u64 = parts[5].parse().unwrap_or(0);
                let write_sectors: u64 = parts[9].parse().unwrap_or(0);
                total_read += read_sectors * 512;
                total_write += write_sectors * 512;
            }
        }
    }
    (total_read, total_write)
}

fn format_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b < 1024.0 {
        format!("{b:.0} B")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else if b < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    } else {
        format!("{:.1} GB", b / 1024.0 / 1024.0 / 1024.0)
    }
}

fn format_rate(bps: f32) -> String {
    if bps < 1024.0 {
        format!("{bps:.0} B/s")
    } else if bps < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.1} MB/s", bps / 1024.0 / 1024.0)
    }
}

fn read_top_processes(limit: usize) -> Vec<ProcessInfo> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    let mut procs = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name_str) = file_name.to_str() else {
            continue;
        };

        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let path = entry.path();
        let Ok(statm) = fs::read_to_string(path.join("statm")) else {
            continue;
        };

        let rss_pages: u64 = statm
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let mem_kb = rss_pages * 4;
        let mem_mb = mem_kb as f32 / 1024.0;

        if mem_mb < 10.0 {
            continue;
        }

        let comm = fs::read_to_string(path.join("comm"))
            .unwrap_or_else(|_| String::from("unknown"))
            .trim()
            .to_string();

        procs.push(ProcessInfo {
            name: comm,
            mem_mb,
        });
    }

    procs.sort_by(|a, b| b.mem_mb.partial_cmp(&a.mem_mb).unwrap_or(std::cmp::Ordering::Equal));
    procs.truncate(limit);
    procs
}

pub fn ipc(arguments: &[String]) -> String {
    for arg in arguments {
        match arg.as_str() {
            "1" | "option1" => SysmonData::set_mode(1),
            "2" | "option2" => SysmonData::set_mode(2),
            "3" | "option3" => SysmonData::set_mode(3),
            "4" | "option4" => SysmonData::set_mode(4),
            _ => {}
        }
    }

    let monitor_arg = arguments.iter().find(|arg| !matches!(arg.as_str(), "toggle" | "show" | "hide" | "1" | "2" | "3" | "4" | "option1" | "option2" | "option3" | "option4")).map(String::as_str);
    Overlay::toggle_sysmon(monitor_arg);
    String::from("ok")
}
