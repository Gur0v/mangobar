mod clock;
mod layout;
mod mango_ipc;
mod settings;
mod status;
mod tags;
mod volume;

use gtk::glib::{self, ControlFlow, LogLevel, LogWriterOutput, Propagation};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Button, CssProvider, EventControllerScroll,
    EventControllerScrollFlags, Label, Orientation,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use mango_ipc::MangoEvent;
use settings::*;
use std::cell::RefCell;
use std::env;
use std::process::Command as StdCommand;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::runtime::Handle;
use tokio::sync::watch;

pub use status::LayoutState;
pub use tags::Tag;

#[derive(Clone, Debug, Default)]
struct Args {
    output: Option<String>,
}

fn main() -> glib::ExitCode {
    quiet_gtk();
    install_log_filter();

    let args = parse_args();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("failed to start tokio runtime");
    let handle = runtime.handle().clone();

    std::thread::spawn(move || runtime.block_on(std::future::pending::<()>()));

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| build_ui(app, args.clone(), handle.clone()));
    app.run()
}

fn quiet_gtk() {
    unsafe {
        env::set_var("GTK_USE_PORTAL", "0");
        env::set_var("GSK_RENDERER", "cairo");
        env::remove_var("G_MESSAGES_DEBUG");
        env::remove_var("GDK_DEBUG");
        env::remove_var("GSK_DEBUG");
    }
}

fn install_log_filter() {
    glib::log_set_writer_func(|level, fields| {
        if matches!(level, LogLevel::Debug | LogLevel::Info) {
            return LogWriterOutput::Handled;
        }

        let mut domain = None;
        let mut message = None;

        for field in fields {
            match field.key() {
                "GLIB_DOMAIN" => domain = field.value_str(),
                "MESSAGE" => message = field.value_str(),
                _ => {}
            }
        }

        if level == LogLevel::Warning
            && domain == Some("Gdk")
            && message.is_some_and(|msg| {
                msg.contains("Cannot get portal org.freedesktop.portal.Inhibit version")
            })
        {
            return LogWriterOutput::Handled;
        }

        glib::log_writer_standard_streams(level, fields)
    });
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut iter = env::args().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-o" | "--output" => args.output = iter.next(),
            "-h" | "--help" => {
                println!("Usage: mangobar [--output <name>]");
                std::process::exit(0);
            }
            unknown => eprintln!("mangobar: ignoring unknown argument: {unknown}"),
        }
    }

    args
}

fn build_ui(app: &Application, args: Args, handle: Handle) {
    install_css();

    let width = monitor_width();
    let (ipc_tx, ipc_rx) = mpsc::channel::<MangoEvent>();
    let (status_tx, status_rx) = mpsc::channel::<String>();
    let (layout_tx, layout_rx) = watch::channel(LayoutState::UNKNOWN);
    let tags = Rc::new(RefCell::new(Vec::<Tag>::new()));

    let window = build_window(app, width);
    let root = build_root(width);
    let tags_box = gtk::Box::new(Orientation::Horizontal, 0);
    let spacer = gtk::Box::new(Orientation::Horizontal, 0);
    let status_label = Label::new(None);

    tags_box.add_css_class("tags");
    spacer.set_hexpand(true);
    spacer.add_css_class("bar");
    status_label.add_css_class("status");
    status_label.set_halign(gtk::Align::End);

    root.append(&tags_box);
    root.append(&spacer);
    root.append(&status_label);
    add_scroll(&root, tags.clone(), args.output.clone(), handle.clone());

    window.set_child(Some(&root));
    window.present();

    if let Ok(initial) = load_tags(args.output.as_deref()) {
        render_tags(&tags_box, &tags, initial, &args.output, &handle);
    }

    mango_ipc::spawn(args.output.clone(), ipc_tx);
    layout::spawn(&handle, args.output.clone(), layout_tx.clone());
    spawn_status(&handle, layout_rx, status_tx);

    glib::timeout_add_local(Duration::from_millis(UI_TICK_MS), move || {
        while let Ok(event) = ipc_rx.try_recv() {
            match event {
                MangoEvent::Tags(next) => {
                    render_tags(&tags_box, &tags, next, &args.output, &handle)
                }
                MangoEvent::Layout(next) => layout::publish(&layout_tx, next),
            }
        }

        while let Ok(status) = status_rx.try_recv() {
            status_label.set_text(&status);
        }

        ControlFlow::Continue
    });
}

fn build_window(app: &Application, width: i32) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("mangobar")
        .decorated(false)
        .resizable(false)
        .build();

    window.init_layer_shell();
    window.set_namespace(Some("mangobar"));
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_exclusive_zone(BAR_HEIGHT);
    window.set_default_size(width, BAR_HEIGHT);
    window.set_width_request(width);
    window
}

fn build_root(width: i32) -> gtk::Box {
    let root = gtk::Box::new(Orientation::Horizontal, 0);
    root.set_height_request(BAR_HEIGHT);
    root.set_width_request(width);
    root.set_halign(gtk::Align::Fill);
    root.set_hexpand(true);
    root.add_css_class("bar");
    root
}

fn add_scroll(
    root: &gtk::Box,
    tags: Rc<RefCell<Vec<Tag>>>,
    output: Option<String>,
    handle: Handle,
) {
    let scroll = EventControllerScroll::new(
        EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::DISCRETE,
    );

    scroll.connect_scroll(move |_, _, dy| {
        if dy == 0.0 {
            return Propagation::Proceed;
        }

        if let Some(number) = scroll_target(&tags.borrow(), dy < 0.0) {
            switch_tag(handle.clone(), number, output.clone());
            Propagation::Stop
        } else {
            Propagation::Proceed
        }
    });

    root.add_controller(scroll);
}

fn spawn_status(
    handle: &Handle,
    mut layout_rx: watch::Receiver<status::LayoutState>,
    tx: mpsc::Sender<String>,
) {
    handle.spawn(async move {
        let (vol_tx, mut vol_rx) = watch::channel(status::VolumeState::UNKNOWN);
        let (time_tx, mut time_rx) = watch::channel(clock::now());

        clock::spawn(time_tx);
        volume::spawn(vol_tx);

        let mut line = String::with_capacity(32);
        let _ = tx.send(status::render(
            &mut line,
            *vol_rx.borrow(),
            *layout_rx.borrow(),
            *time_rx.borrow(),
        ));

        loop {
            tokio::select! {
                changed = vol_rx.changed() => if changed.is_err() { break; },
                changed = layout_rx.changed() => if changed.is_err() { break; },
                changed = time_rx.changed() => if changed.is_err() { break; },
            }

            let volume = newest(&mut vol_rx);
            let layout = newest(&mut layout_rx);
            let time = newest(&mut time_rx);
            let _ = tx.send(status::render(&mut line, volume, layout, time));
        }
    });
}

fn newest<T: Copy>(rx: &mut watch::Receiver<T>) -> T {
    let value = *rx.borrow_and_update();
    while rx.has_changed().unwrap_or(false) {
        rx.borrow_and_update();
    }
    value
}

fn scroll_target(tags: &[Tag], previous: bool) -> Option<u32> {
    let visible: Vec<&Tag> = tags
        .iter()
        .filter(|tag| tag.active || tag.occupied || tag.urgent)
        .collect();

    if visible.len() < 2 {
        return None;
    }

    let active = visible.iter().position(|tag| tag.active)?;
    let target = if previous {
        active.checked_sub(1)?
    } else if active + 1 < visible.len() {
        active + 1
    } else {
        return None;
    };

    Some(visible[target].number)
}

fn monitor_width() -> i32 {
    let Some(display) = gtk::gdk::Display::default() else {
        return 1920;
    };

    let monitors = display.monitors();
    let Some(monitor) = monitors.item(0).and_downcast::<gtk::gdk::Monitor>() else {
        return 1920;
    };

    monitor.geometry().width()
}

fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_data(&format!(
        "
        window {{ background: {BACKGROUND}; }}

        .bar {{
            background: {BACKGROUND};
            color: {FOREGROUND};
            font: {FONT};
        }}

        .tags {{ margin-left: {LEFT_PADDING}px; }}

        .status {{
            margin-right: {RIGHT_PADDING}px;
            color: {FOREGROUND};
        }}

        button.tag {{
            min-width: {TAG_MIN_WIDTH}px;
            min-height: {TAG_MIN_HEIGHT}px;
            margin: 1px 4px 1px 0;
            padding: 0;
            border: 0;
            border-radius: 0;
            background: {BACKGROUND};
            color: {DIM_FOREGROUND};
        }}

        button.tag.active {{ color: {FOREGROUND}; }}
        button.tag.urgent {{ color: {FOREGROUND}; }}
        "
    ));

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn render_tags(
    tags_box: &gtk::Box,
    last_tags: &Rc<RefCell<Vec<Tag>>>,
    tags: Vec<Tag>,
    output: &Option<String>,
    handle: &Handle,
) {
    if *last_tags.borrow() == tags {
        return;
    }

    while let Some(child) = tags_box.first_child() {
        tags_box.remove(&child);
    }

    for tag in &tags {
        if !tag.active && !tag.occupied && !tag.urgent {
            continue;
        }

        let button = Button::with_label(&tag.number.to_string());
        button.add_css_class("tag");

        if tag.active {
            button.add_css_class("active");
        }
        if tag.urgent {
            button.add_css_class("urgent");
        }

        let number = tag.number;
        let output = output.clone();
        let handle = handle.clone();
        button.connect_clicked(move |_| switch_tag(handle.clone(), number, output.clone()));
        tags_box.append(&button);
    }

    *last_tags.borrow_mut() = tags;
}

fn load_tags(output: Option<&str>) -> Result<Vec<Tag>, String> {
    let mut command = StdCommand::new("mmsg");
    if let Some(output) = output {
        command.args(["-o", output]);
    }
    command.args(["-g", "-t"]);

    let out = command.output().map_err(|err| err.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    parse_tags(&String::from_utf8_lossy(&out.stdout))
}

fn parse_tags(stdout: &str) -> Result<Vec<Tag>, String> {
    let mut tags = Vec::new();

    for line in stdout.lines() {
        let mut fields = line.split_whitespace();
        let Some(first) = fields.next() else { continue };

        if first != "tag" && fields.next() != Some("tag") {
            continue;
        }

        let number = parse_u32(fields.next(), line, "tag number")?;
        let state = parse_u32(fields.next(), line, "tag state")?;
        let clients = parse_u32(fields.next(), line, "client count")?;
        let focused = parse_u32(fields.next(), line, "focused client")?;

        tags.push(Tag {
            number,
            active: state == 1,
            urgent: state == 2,
            occupied: clients > 0,
            focused_client: focused != 0,
        });
    }

    tags.sort_by_key(|tag| tag.number);
    tags.dedup_by(|a, b| {
        if a.number != b.number {
            return false;
        }

        b.active |= a.active;
        b.urgent |= a.urgent;
        b.occupied |= a.occupied;
        b.focused_client |= a.focused_client;
        true
    });

    if tags.is_empty() {
        Err("mmsg returned no tag lines".to_string())
    } else {
        Ok(tags)
    }
}

fn parse_u32(value: Option<&str>, line: &str, field: &str) -> Result<u32, String> {
    value
        .ok_or_else(|| format!("missing {field} in `{line}`"))?
        .parse::<u32>()
        .map_err(|err| format!("invalid {field} in `{line}`: {err}"))
}

fn switch_tag(handle: Handle, number: u32, output: Option<String>) {
    handle.spawn(async move {
        let mut command = TokioCommand::new("mmsg");
        if let Some(output) = output.as_deref() {
            command.args(["-o", output]);
        }
        command.args(["-s", "-t", &number.to_string()]);

        match command.output().await {
            Ok(out) if out.status.success() => {}
            Ok(out) => eprintln!(
                "mangobar: failed to switch to tag {number}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(err) => eprintln!("mangobar: failed to switch to tag {number}: {err}"),
        }
    });
}
