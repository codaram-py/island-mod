# codabar

A taskbar that replaces the Windows one, rather than floating over it.

## How it takes the edge

`SHAppBarMessage` reserves a strip of the screen, so a maximized window stops
above the bar the way it stopped above the shell's taskbar. Nothing is set
topmost and the z-order is left alone — this is not an overlay.

The shell's own taskbar is hidden, never closed, and only once the reservation
is in hand, so the edge is never unowned. Hiding its window does not retract
its appbar reservation, so `ABM_QUERYPOS` -- which exists to slide a new appbar
clear of the ones already there -- would park us one taskbar height up the
screen. While replacing it we claim the edge outright and skip that
negotiation; with `replaceShell` off we honour it, because then we really are
sharing the screen. Putting it back is covered three ways:
`Drop` on the ordinary exit, a panic hook, and a marker file in
`%LOCALAPPDATA%\codabar` that lets the next run undo a hide left behind by a
crash. Turning off **Substituir a barra do Windows** in the settings leaves the
shell bar on screen and only reserves the edge, which is the safe way to try it.

`geom.rs` holds the rectangle arithmetic with no win32 in it, so the part that
decides whether the bar lands on the screen edge or hovering above it is
covered by tests that run anywhere: `cargo test`.

## Building

Needs the MSVC toolchain and the WebView2 runtime (ships with Windows 11).

```
cargo install tauri-cli --version "^2"
cd src-tauri
cargo tauri build
```

The installer lands in `src-tauri/target/release/bundle/nsis/`. For a run
without packaging, `cargo tauri dev`.

The CI container this was written in has no MSVC linker, so the code is
verified with `cargo check --target x86_64-pc-windows-msvc` and built on
Windows.

## Settings

`%LOCALAPPDATA%\codabar\config.json`, or the gear on the bar. Edge, thickness,
colours, transparency, blur, corner radius, margin, font and which widgets show.
The bar ships as acrylic at 86%, which is roughly what Windows 11's own
taskbar looks like; turn the opacity down for a see-through one, bearing in
mind that then you see whatever sits behind it. Changes apply live; the file is written through a temp file so a crash mid-write
cannot truncate it.

## Widgets

Working: Start, search, task view, the open windows on the bar's monitor,
volume (click mutes, wheel adjusts), network (wi-fi SSID and signal from the
WLAN service, falling back to the network list manager for a wired link),
microphone mute, battery, clock, show desktop, the Claude Code button,
settings.

Start, search, task view and show desktop have no public API worth the
trouble, so the bar taps the Win chord the user would have typed. Battery
hides itself on a machine that has none.

The window list filters the way the shell's own taskbar does -- visible,
titled, unowned, not a tool window, and not DWM-cloaked, which is what keeps
suspended UWP apps and phantom `ApplicationFrame` windows out. Icons come from
`WM_GETICON`, then the window class, then the executable, and are cached per
window until the process behind it changes. Click focuses, middle-click closes.

Not yet: the shell's notification area, VPN, music, per-app mixer. They appear
greyed out in the settings rather than as widgets that do nothing.

The background-apps tray is the hard one. Tray icons belong to the shell: apps
call `Shell_NotifyIcon`, which finds `Shell_TrayWnd` and sends `WM_COPYDATA` to
Explorer. Becoming the tray server breaks Explorer's own tray and loses every
icon registered before we started, so the plan is to mirror instead — read the
buttons out of `TrayNotifyWnd` → `ToolbarWindow32` with `TB_GETBUTTON` across
the process boundary, redraw them, and forward clicks back. That is
version-sensitive and will need iterating against a real Windows 11 build.
