use crate::{LayoutState, Tag};
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use wayland_client::protocol::{wl_output, wl_registry};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

pub mod dwl_ipc {
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/dwl-ipc-unstable-v2.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocols/dwl-ipc-unstable-v2.xml");
}

use dwl_ipc::zdwl_ipc_manager_v2::{self, ZdwlIpcManagerV2};
use dwl_ipc::zdwl_ipc_output_v2::{self, ZdwlIpcOutputV2};

#[derive(Clone, Debug)]
pub enum MangoEvent {
    Tags(Vec<Tag>),
    Layout(LayoutState),
}

struct OutputData {
    global_name: u32,
}

#[derive(Default)]
struct OutputState {
    wl_output: Option<wl_output::WlOutput>,
    name: Option<String>,
    ipc_bound: bool,
}

struct IpcState {
    tx: mpsc::Sender<MangoEvent>,
    output_filter: Option<String>,
    manager: Option<ZdwlIpcManagerV2>,
    outputs: HashMap<u32, OutputState>,
    pending_tags: HashMap<String, Vec<Tag>>,
}

pub fn spawn(output_filter: Option<String>, tx: mpsc::Sender<MangoEvent>) {
    thread::spawn(move || {
        loop {
            if let Err(err) = run(output_filter.clone(), tx.clone()) {
                eprintln!("mangobar: MangoWM direct IPC failed: {err}");
                thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    });
}

fn run(output_filter: Option<String>, tx: mpsc::Sender<MangoEvent>) -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|err| err.to_string())?;
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, ());

    let mut state = IpcState {
        tx,
        output_filter,
        manager: None,
        outputs: HashMap::new(),
        pending_tags: HashMap::new(),
    };

    event_queue
        .roundtrip(&mut state)
        .map_err(|err| err.to_string())?;
    state.bind_ready_outputs(&qh);
    event_queue
        .roundtrip(&mut state)
        .map_err(|err| err.to_string())?;
    loop {
        event_queue
            .blocking_dispatch(&mut state)
            .map_err(|err| err.to_string())?;
    }
}

impl IpcState {
    fn try_bind_output(&mut self, global_name: u32, qh: &QueueHandle<Self>) {
        let Some(manager) = self.manager.as_ref() else {
            return;
        };
        let Some(output) = self.outputs.get_mut(&global_name) else {
            return;
        };
        if output.ipc_bound {
            return;
        }
        let Some(name) = output.name.as_ref() else {
            return;
        };
        if self
            .output_filter
            .as_deref()
            .is_some_and(|filter| filter != name)
        {
            return;
        }
        let Some(wl_output) = output.wl_output.as_ref() else {
            return;
        };

        manager.get_output(wl_output, qh, name.clone());
        output.ipc_bound = true;
    }

    fn bind_ready_outputs(&mut self, qh: &QueueHandle<Self>) {
        let keys: Vec<u32> = self.outputs.keys().copied().collect();
        for key in keys {
            self.try_bind_output(key, qh);
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for IpcState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == wl_output::WlOutput::interface().name {
                let wl_output = registry.bind::<wl_output::WlOutput, _, _>(
                    name,
                    version.min(4),
                    qh,
                    OutputData { global_name: name },
                );
                state.outputs.entry(name).or_default().wl_output = Some(wl_output);
            } else if interface == ZdwlIpcManagerV2::interface().name {
                let manager = registry.bind::<ZdwlIpcManagerV2, _, _>(name, version.min(2), qh, ());
                state.manager = Some(manager);
                state.bind_ready_outputs(qh);
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, OutputData> for IpcState {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        data: &OutputData,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            state.outputs.entry(data.global_name).or_default().name = Some(name);
            state.try_bind_output(data.global_name, qh);
        }
    }
}

impl Dispatch<ZdwlIpcManagerV2, ()> for IpcState {
    fn event(
        _: &mut Self,
        _: &ZdwlIpcManagerV2,
        _: zdwl_ipc_manager_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZdwlIpcOutputV2, String> for IpcState {
    fn event(
        state: &mut Self,
        _: &ZdwlIpcOutputV2,
        event: zdwl_ipc_output_v2::Event,
        output_name: &String,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zdwl_ipc_output_v2::Event::Tag {
                tag,
                state: tag_state,
                clients,
                focused,
            } => {
                let tags = state.pending_tags.entry(output_name.clone()).or_default();
                tags.push(Tag {
                    number: tag + 1,
                    active: tag_state
                        .into_result()
                        .ok()
                        .is_some_and(|state| state == zdwl_ipc_output_v2::TagState::Active),
                    urgent: tag_state
                        .into_result()
                        .ok()
                        .is_some_and(|state| state == zdwl_ipc_output_v2::TagState::Urgent),
                    occupied: clients > 0,
                    focused_client: focused != 0,
                });
            }
            zdwl_ipc_output_v2::Event::KbLayout { kb_layout } => {
                let _ = state
                    .tx
                    .send(MangoEvent::Layout(LayoutState::from_name(&kb_layout)));
            }
            zdwl_ipc_output_v2::Event::Frame => {
                if let Some(mut tags) = state.pending_tags.remove(output_name) {
                    tags.sort_by_key(|tag| tag.number);
                    if tags
                        .iter()
                        .any(|tag| tag.active || tag.occupied || tag.urgent)
                    {
                        let _ = state.tx.send(MangoEvent::Tags(tags));
                    }
                }
            }
            _ => {}
        }
    }
}
