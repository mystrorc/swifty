use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use amane::Service;
use crate::settings::Settings;

static SHAKING: AtomicBool = AtomicBool::new(false);

// Tuned parameters for shake detection (reduced sensitivity to prevent accidental triggers)
const MIN_PACKET_DELTA: i32 = 6;
const REVERSAL_WINDOW_MS: u64 = 300;
const IDLE_RESET_MS: u64 = 350;
const REQUIRED_REVERSALS: u32 = 4;
const REQUIRED_DISTANCE: i32 = 75;

pub struct ShakeCursor;

impl Service for ShakeCursor {
    fn new() -> Self {
        Self
    }

    fn listen() {
        thread::spawn(run_listener);
    }
}

fn run_listener() {
    let mut buffer = [0u8; 48];

    // Retry loop in case /dev/input/mice is temporarily unavailable
    loop {
        let mut file = match File::open("/dev/input/mice") {
            Ok(f) => f,
            Err(_) => {
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        let mut x_reversals: u32 = 0;
        let mut last_x_sign: i8 = 0;
        let mut last_x_reversal = Instant::now();
        let mut accumulated_dx: i32 = 0;

        let mut y_reversals: u32 = 0;
        let mut last_y_sign: i8 = 0;
        let mut last_y_reversal = Instant::now();
        let mut accumulated_dy: i32 = 0;
        let mut last_setting_check = Instant::now() - Duration::from_secs(1);
        let mut shake_enabled = false;

        while let Ok(count) = file.read(&mut buffer) {
            if count < 3 {
                continue;
            }

            let now = Instant::now();
            if now.duration_since(last_setting_check) >= Duration::from_millis(500) {
                shake_enabled = Settings::read().flag("shake_cursor");
                last_setting_check = now;
            }

            if !shake_enabled {
                continue;
            }

            let mut shake_detected = false;
            let now = Instant::now();

            if now.duration_since(last_x_reversal) > Duration::from_millis(IDLE_RESET_MS) {
                x_reversals = 0;
                accumulated_dx = 0;
            }
            if now.duration_since(last_y_reversal) > Duration::from_millis(IDLE_RESET_MS) {
                y_reversals = 0;
                accumulated_dy = 0;
            }

            // PS/2 mouse packet is 3 bytes: [flags, dx, dy]
            for chunk in buffer[..count].chunks_exact(3) {
                let dx = chunk[1] as i8 as i32;
                let dy = chunk[2] as i8 as i32;
                let packet_time = Instant::now();

                // Horizontal movement
                if dx.abs() >= MIN_PACKET_DELTA {
                    let sign = if dx > 0 { 1 } else { -1 };
                    if sign == last_x_sign {
                        accumulated_dx += dx.abs();
                    } else {
                        if packet_time.duration_since(last_x_reversal) < Duration::from_millis(REVERSAL_WINDOW_MS) {
                            x_reversals += 1;
                            accumulated_dx += dx.abs();
                            if x_reversals >= REQUIRED_REVERSALS && accumulated_dx >= REQUIRED_DISTANCE {
                                shake_detected = true;
                                break;
                            }
                        } else {
                            x_reversals = 1;
                            accumulated_dx = dx.abs();
                        }
                        last_x_sign = sign;
                        last_x_reversal = packet_time;
                    }
                }

                // Vertical movement
                if dy.abs() >= MIN_PACKET_DELTA {
                    let sign = if dy > 0 { 1 } else { -1 };
                    if sign == last_y_sign {
                        accumulated_dy += dy.abs();
                    } else {
                        if packet_time.duration_since(last_y_reversal) < Duration::from_millis(REVERSAL_WINDOW_MS) {
                            y_reversals += 1;
                            accumulated_dy += dy.abs();
                            if y_reversals >= REQUIRED_REVERSALS && accumulated_dy >= REQUIRED_DISTANCE {
                                shake_detected = true;
                                break;
                            }
                        } else {
                            y_reversals = 1;
                            accumulated_dy = dy.abs();
                        }
                        last_y_sign = sign;
                        last_y_reversal = packet_time;
                    }
                }
            }

            if shake_detected {
                x_reversals = 0;
                accumulated_dx = 0;
                y_reversals = 0;
                accumulated_dy = 0;

                if !SHAKING.load(Ordering::SeqCst) {
                    SHAKING.store(true, Ordering::SeqCst);
                    thread::spawn(trigger_shake_effect);
                }
            }
        }

        thread::sleep(Duration::from_secs(1));
    }
}

fn trigger_shake_effect() {
    let settings = Settings::read();
    let theme = crate::settings::active_cursor_theme();
    let normal_size = if settings.text("cursor_size").is_empty() {
        "24".to_string()
    } else {
        settings.text("cursor_size").to_string()
    };
    drop(settings);

    let home = env::var("HOME").unwrap_or_default();
    let cursor_kdl = format!("{home}/.config/niri/cursor.kdl");

    // Temporarily enlarge cursor to 64px
    let enlarged_config = format!(
        "cursor {{\n    xcursor-theme \"{theme}\"\n    xcursor-size 64\n}}\n"
    );
    let _ = fs::write(&cursor_kdl, enlarged_config);
    let _ = Command::new("niri").arg("msg").arg("action").arg("load-config-file").status();

    thread::sleep(Duration::from_millis(1200));

    // Restore normal size
    let normal_config = format!(
        "cursor {{\n    xcursor-theme \"{theme}\"\n    xcursor-size {normal_size}\n}}\n"
    );
    let _ = fs::write(&cursor_kdl, normal_config);
    let _ = Command::new("niri").arg("msg").arg("action").arg("load-config-file").status();

    SHAKING.store(false, Ordering::SeqCst);
}
