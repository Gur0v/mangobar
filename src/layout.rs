use crate::status::LayoutState;
use std::time::Duration;
use tokio::process::Command;
use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::time::{MissedTickBehavior, interval};

pub fn spawn(handle: &Handle, output: Option<String>, tx: watch::Sender<LayoutState>) {
    handle.spawn(async move {
        let mut tick = interval(Duration::from_millis(100));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            if let Ok(Ok(layout)) =
                tokio::time::timeout(Duration::from_millis(80), current(output.as_deref())).await
            {
                publish(&tx, layout);
            }
        }
    });
}

pub fn publish(tx: &watch::Sender<LayoutState>, next: LayoutState) {
    let _ = tx.send_if_modified(|current| {
        if *current == next {
            false
        } else {
            *current = next;
            true
        }
    });
}

async fn current(output: Option<&str>) -> Result<LayoutState, String> {
    let mut command = Command::new("mmsg");
    if let Some(output) = output {
        command.args(["-o", output]);
    }
    command.args(["-g", "-k"]);

    let out = command.output().await.map_err(|err| err.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    parse(&String::from_utf8_lossy(&out.stdout))
}

fn parse(stdout: &str) -> Result<LayoutState, String> {
    for line in stdout.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(index) = fields.iter().position(|field| *field == "kb_layout") else {
            continue;
        };
        if index + 1 >= fields.len() {
            continue;
        }
        return Ok(LayoutState::from_name(&fields[index + 1..].join(" ")));
    }

    Err("mmsg returned no keyboard layout line".to_string())
}
