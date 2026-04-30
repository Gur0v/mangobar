use crate::status::VolumeState;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::watch;
use tokio::time::{MissedTickBehavior, interval};

pub fn spawn(tx: watch::Sender<VolumeState>) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(100));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            let next = tokio::time::timeout(Duration::from_millis(80), current())
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or(VolumeState::UNKNOWN);

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

async fn current() -> Result<VolumeState, String> {
    let out = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .await
        .map_err(|err| err.to_string())?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    parse(&String::from_utf8_lossy(&out.stdout))
}

fn parse(stdout: &str) -> Result<VolumeState, String> {
    let muted = stdout.contains("[MUTED]");
    let raw = stdout
        .split_whitespace()
        .find_map(|field| field.parse::<f32>().ok())
        .ok_or_else(|| format!("missing volume in `{}`", stdout.trim()))?;
    let percent = (raw * 100.0).round().clamp(0.0, 999.0) as u16;
    Ok(VolumeState::new(percent, muted))
}
