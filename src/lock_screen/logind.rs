use std::process;
use std::thread;

use amane::{Argument, Bus, Service};

use super::curtain;
use super::password::Typing;

const LOGIN1: &str = "org.freedesktop.login1";
const MANAGER_PATH: &str = "/org/freedesktop/login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const SESSION: &str = "org.freedesktop.login1.Session";

/*
 * logind asks the session's locker to lock through a signal, which is what
 * `loginctl lock-session`, idle daemons and a closed lid all go through
 */
pub struct Logind;

impl Service for Logind {
    fn new() -> Self {
        Self
    }

    fn listen() {
        let bus = Bus::system();

        let pid = Argument::Unsigned(process::id());

        let session = bus.call(LOGIN1, MANAGER_PATH, MANAGER, "GetSessionByPID", &[pid]);

        // outside a logind session, any session's lock request is taken
        let session = String::from(session.text());

        let unlock_session = session.clone();

        thread::spawn(move || {
            let bus = Bus::system();

            for signal in bus.signals(SESSION, "Unlock") {
                if !unlock_session.is_empty() && signal.path() != unlock_session {
                    continue;
                }

                amane::Lock::force_unlock();
                curtain::open();
            }
        });

        for signal in bus.signals(SESSION, "Lock") {
            if !session.is_empty() && signal.path() != session {
                continue;
            }

            start();
        }
    }
}

// every lock opens on the hint, not on the last lock's half typed state; blocks while the desktop fades
pub fn start() {
    Typing::write().typing = false;

    curtain::close();
}
