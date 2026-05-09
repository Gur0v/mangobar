# mangobar

A suckless-esc status bar for mangowc with swaybar energy.

![Preview](PREVIEW.png)

Black strip. Workspace numbers on the left. Volume, layout, and clock on the right. No config format. Patch the source.

```text
42% us 2026-04-24 09:49:57 PM
```

## What It Is

mangobar is a small Rust bar for mangowc. It shows:

- mangowc tags/workspaces on the left (vacant tags hidden)
- volume, keyboard layout, and clock on the right
- a plain black background
- no extra visual noise

There is no separate `status_command`. The status lives inside the bar.

## How It Works

- `src/mango_ipc.rs` talks to mangowc through `dwl-ipc-unstable-v2` for workspace updates.
- `src/layout.rs` polls `mmsg -g -k` for keyboard layout because mangowc does not currently emit layout changes reliably.
- `src/volume.rs` uses `wpctl get-volume @DEFAULT_AUDIO_SINK@` for volume.
- `src/clock.rs` updates the clock once per second.
- `src/status.rs` renders the right-side status text.
- `src/settings.rs` contains the values people are expected to tweak first.
- `src/main.rs` handles GTK, layer-shell, rendering, clicks, and scroll switching.

See [PATCHING.md](PATCHING.md) for a full source map and guide to modifying the bar.

## Controls

- Click a workspace number to switch to it.
- Scroll over the bar to move between visible workspaces.
- Vacant tags are hidden.

## Build

Void dependencies:

```sh
sudo xbps-install -S rust cargo gtk4 gtk4-devel gtk4-layer-shell gtk4-layer-shell-devel gdk-pixbuf gdk-pixbuf-devel wireplumber
```

You also need mangowc's `mmsg` in `PATH`.

```sh
cargo build --release
# or
make build
```

Useful development commands:

```sh
make fmt
make check
make run
make clean
```

## Install

```sh
sudo install -Dm755 target/release/mangobar /usr/local/bin/mangobar
# or
sudo make install
```

## Run

Inside mangowc:

```sh
./target/release/mangobar
```

For one output:

```sh
./target/release/mangobar --output DP-1
```

## Scope

Supported:

- mangowc (and compatible `dwl-ipc-unstable-v2` compositors)
- GTK4 + Wayland layer-shell
- PipeWire/WirePlumber via `wpctl`

Not the goal:

- a general-purpose bar framework
- a theme engine or config format
- a widget garden
- supporting every compositor out of the box

## License

[GPL-3.0](LICENSE)
