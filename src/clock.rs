use crate::status::ClockState;
use libc::{localtime_r, strftime, time, time_t, tm};
use std::mem::MaybeUninit;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::{MissedTickBehavior, interval};

const CLOCK_FORMAT: &[u8] = b"%Y-%m-%d %I:%M:%S %p\0";
const CLOCK_LEN: usize = 22;

pub fn now() -> ClockState {
    let mut raw: time_t = 0;
    let mut local = MaybeUninit::<tm>::uninit();
    let mut buf = [0u8; CLOCK_LEN + 1];

    unsafe {
        time(&mut raw);

        if localtime_r(&raw, local.as_mut_ptr()).is_null() {
            return ClockState::from_bytes(*b"1970-01-01 12:00:00 AM", CLOCK_LEN as u8);
        }

        let written = strftime(
            buf.as_mut_ptr().cast(),
            buf.len(),
            CLOCK_FORMAT.as_ptr().cast(),
            local.as_ptr(),
        );

        if written == 0 || written > CLOCK_LEN {
            return ClockState::from_bytes(*b"1970-01-01 12:00:00 AM", CLOCK_LEN as u8);
        }
    }

    let mut out = [0u8; CLOCK_LEN];
    out[..CLOCK_LEN].copy_from_slice(&buf[..CLOCK_LEN]);
    ClockState::from_bytes(out, CLOCK_LEN as u8)
}

pub fn spawn(tx: watch::Sender<ClockState>) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            let next = now();
            let _ = tx.send_if_modified(|current| {
                if *current == next {
                    false
                } else {
                    *current = next;
                    true
                }
            });
        }
    });
}
