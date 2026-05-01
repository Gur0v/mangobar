# mangobar

A tiny `mangowc` bar with swaybar energy.

`mangobar` is my attempt at recreating the parts of `swaybar` I actually used with my simple config: a small black strip, workspace numbers on the left, and a compact status line on the right.

That is the whole vibe.

## What It Is

`mangobar` is a small Rust bar for mangowc.

It shows:

- mangowc tags/workspaces on the left
- volume, keyboard layout, and clock on the right
- a plain black background
- no extra visual noise

The right side is basically a less-native [`i3status-dumb`](https://github.com/Gur0v/i3status-dumb) built into the bar.

It renders this kind of line:

```text
42% us 2026-04-24 09:49:57 PM
```

There is no separate `status_command`. The status lives inside the bar.

## Backstory

After switching to mangowc, I started missing the simple look of `swaybar`.

So I tried the bars people usually use on wlroots window managers.

Waybar, yambar, ironbar, and the usual suspects are cool. They can do a lot. They just did not scratch that itch.

Black strip. Workspace numbers. Tiny status text. Nothing trying to be the center of attention.

So this is a small bar for mangowc that tries to feel like my old swaybar setup, plus a built-in status line inspired by `i3status-dumb`.

## How It Works

- `src/mango_ipc.rs` talks to mangowc through `dwl-ipc-unstable-v2` for workspace updates.
- `src/layout.rs` polls `mmsg -g -k` for keyboard layout because mangowc does not currently emit layout changes reliably.
- `src/volume.rs` uses `wpctl get-volume @DEFAULT_AUDIO_SINK@` for volume.
- `src/clock.rs` updates the clock once per second.
- `src/status.rs` renders the right-side status text.
- `src/settings.rs` contains the obvious values people are expected to tweak first.
- `src/main.rs` handles GTK, layer-shell, rendering, clicks, and scroll switching.

## Patching

This project is meant to be patched, not configured.

The idea is to keep the default code small and readable enough that changing it is easier than inventing a config format.

Start here:

- `src/settings.rs`
  Change bar height, font, colors, padding, tag size, and polling intervals.
- `src/status.rs`
  Change the right-side status format. This is where `volume layout clock` becomes whatever order or text you want.
- `src/volume.rs`
  Change how volume is read. Right now it shells out to `wpctl`.
- `src/layout.rs`
  Change keyboard layout behavior. Right now it polls `mmsg -g -k` because the watch path does not update for me.
- `src/clock.rs`
  Change the clock format or tick behavior.
- `src/main.rs`
  Change GTK layout, CSS classes, click behavior, scroll behavior, and workspace rendering.
- `src/mango_ipc.rs`
  Change direct mangowc IPC behavior. Most people should not need to touch this unless they want more compositor state.

Common tweaks:

- Bar height: `BAR_HEIGHT` in `src/settings.rs`
- Font: `FONT` in `src/settings.rs`
- Background/text colors: `BACKGROUND`, `FOREGROUND`, and `DIM_FOREGROUND` in `src/settings.rs`
- Workspace spacing: `LEFT_PADDING`, `TAG_MIN_WIDTH`, and `TAG_MIN_HEIGHT` in `src/settings.rs`
- Status text order: `render()` in `src/status.rs`
- Volume refresh speed: `VOLUME_INTERVAL_MS` and `VOLUME_TIMEOUT_MS` in `src/settings.rs`
- Layout refresh speed: `LAYOUT_INTERVAL_MS` and `LAYOUT_TIMEOUT_MS` in `src/settings.rs`

The CSS is generated in `src/main.rs` from values in `src/settings.rs`, so basic visual tweaks should not require digging through a giant stylesheet.

## Source Map

If you want to patch one thing, start here:

| Goal | File |
| :-- | :-- |
| Change colors/font/height/spacing | `src/settings.rs` |
| Change workspace visibility or click behavior | `src/main.rs` |
| Change scroll behavior | `src/main.rs` |
| Change status order/text | `src/status.rs` |
| Change volume command/parsing | `src/volume.rs` |
| Change keyboard layout command/parsing | `src/layout.rs` |
| Change clock format | `src/clock.rs` |
| Add more mangowc IPC state | `src/mango_ipc.rs` |
| Update generated IPC protocol | `protocols/dwl-ipc-unstable-v2.xml` |

Recommended patch flow:

```sh
make fmt
make check
cargo build --release
```

## Controls

- Click a workspace number to switch to it.
- Scroll over the bar to move between visible workspaces.
- Vacant tags are hidden.

## Scope

Supported:

- mangowc
- GTK4 layer-shell
- PipeWire/WirePlumber through `wpctl`
- my simple swaybar-ish setup

Not the goal:

- a general-purpose bar framework
- a theme engine
- JSON status protocols
- a widget garden
- supporting every compositor under the sun

## Build

Void dependencies:

```sh
sudo xbps-install -S rust cargo gtk4 gtk4-devel gtk4-layer-shell gtk4-layer-shell-devel gdk-pixbuf gdk-pixbuf-devel wireplumber
```

You also need mangowc's `mmsg` in `PATH`.

Build:

```sh
cargo build --release
```

Or:

```sh
make build
```

Install:

```sh
sudo install -Dm755 target/release/mangobar /usr/local/bin/mangobar
```

Or:

```sh
sudo make install
```

Useful development commands:

```sh
make fmt
make check
make run
make clean
```

Binary:

```text
target/release/mangobar
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

## Notes

- Workspace updates use direct mangowc IPC.
- Workspace switching uses `mmsg -s -t`.
- Keyboard layout uses fast bounded `mmsg -g -k` polling because the watch/event path does not update properly right now.
- Volume uses `wpctl`, not `pactl`.

## License

[GPL-3.0](LICENSE)
