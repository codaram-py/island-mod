#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appbar;
mod geom;
#[cfg(windows)]
mod audio;
mod config;
#[cfg(windows)]
mod kbd;
#[cfg(windows)]
mod power;
#[cfg(windows)]
mod net;
#[cfg(windows)]
mod winlist;

use std::sync::Mutex;

use appbar::{AppBar, Edge};
use config::Config;
use tauri::{Emitter, Manager, RunEvent};

fn edge_from(s: &str) -> Edge {
    match s {
        "top" => Edge::Top,
        "left" => Edge::Left,
        "right" => Edge::Right,
        _ => Edge::Bottom,
    }
}

/// Holds the screen reservation for as long as the app runs.
///
/// `AppBar` carries an `HWND`, which is a raw pointer and so not `Send` by
/// default. Tauri runs commands on a pool, so the handle has to cross threads:
/// that is sound here because the only win32 calls made through it --
/// `SHAppBarMessage` and `SetWindowPos` -- are callable from any thread, and
/// the `Mutex` keeps them from overlapping.
struct BarState {
    bar: Mutex<Option<AppBar>>,
    config: Mutex<Config>,
    /// The monitor the bar sits on and the bar's own window, so the app list
    /// can be scoped to this screen and leave itself out.
    #[cfg(windows)]
    monitor: Mutex<(winlist::BarMonitor, winlist::BarHwnd)>,
}

unsafe impl Send for BarState {}
unsafe impl Sync for BarState {}

#[tauri::command]
fn get_config(state: tauri::State<'_, BarState>) -> Config {
    state.config.lock().unwrap().clone()
}

/// Persist a config and apply the parts the window itself owns.
///
/// The theme is the frontend's business; only edge and thickness move the
/// window, so those are the only ones that touch the appbar.
#[tauri::command]
fn set_config(mut next: Config, state: tauri::State<'_, BarState>) -> Result<Config, String> {
    next.sanitise();
    next.save().map_err(|e| e.to_string())?;

    let previous = {
        let mut held = state.config.lock().unwrap();
        let was = held.clone();
        *held = next.clone();
        was
    };

    if previous.edge != next.edge || previous.thickness != next.thickness {
        if let Some(bar) = state.bar.lock().unwrap().as_mut() {
            bar.set_edge(edge_from(&next.edge));
            bar.set_thickness(next.thickness as i32);
        }
    }
    Ok(next)
}

/// Put every setting back to the defaults, keeping the file in step.
#[tauri::command]
fn reset_config(state: tauri::State<'_, BarState>) -> Result<Config, String> {
    let fresh = Config::default();
    fresh.save().map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = fresh.clone();
    if let Some(bar) = state.bar.lock().unwrap().as_mut() {
        bar.set_edge(edge_from(&fresh.edge));
        bar.set_thickness(fresh.thickness as i32);
    }
    Ok(fresh)
}

/// What the bar can tell the settings window about the shell it replaced.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    edge: String,
    horizontal: bool,
    shell_visible: bool,
}

#[tauri::command]
fn bar_status(state: tauri::State<'_, BarState>) -> Status {
    let held = state.bar.lock().unwrap();
    let (edge, horizontal) = match held.as_ref() {
        Some(bar) => (format!("{:?}", bar.edge()).to_lowercase(), bar.horizontal()),
        None => (state.config.lock().unwrap().edge.clone(), true),
    };
    Status {
        edge,
        horizontal,
        shell_visible: appbar::shell_visible(),
    }
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("settings")
        .ok_or("the settings window is missing from tauri.conf.json")?;
    w.show().map_err(|e| e.to_string())?;
    w.unminimize().ok();
    w.set_focus().map_err(|e| e.to_string())
}

/// Open Claude Code in its own window.
///
/// Spawned detached through the shell so a GUI build with no console does not
/// flash one, and so the bar is not holding the child's handles.
#[tauri::command]
fn launch_claude(state: tauri::State<'_, BarState>) -> Result<(), String> {
    let cmd = state.config.lock().unwrap().widgets.claude_command.clone();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &cmd])
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
        Err("only on windows".into())
    }
}

/// Tap Win, optionally with another key held, to reach a shell surface.
///
/// The Start menu, task view, search and show-desktop all answer to a Win
/// chord and have no public API worth the trouble, so this is how the bar
/// reaches them -- the same keystroke the user would type.
#[cfg(windows)]
fn tap_win(with: Option<u16>) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_LWIN};
    unsafe {
        let win = VK_LWIN.0 as u8;
        keybd_event(win, 0, Default::default(), 0);
        if let Some(k) = with {
            keybd_event(k as u8, 0, Default::default(), 0);
            keybd_event(k as u8, 0, KEYEVENTF_KEYUP, 0);
        }
        keybd_event(win, 0, KEYEVENTF_KEYUP, 0);
    }
}

#[tauri::command]
fn open_start_menu() -> Result<(), String> {
    #[cfg(windows)]
    tap_win(None);
    Ok(())
}

#[tauri::command]
fn open_task_view() -> Result<(), String> {
    #[cfg(windows)]
    tap_win(Some(windows::Win32::UI::Input::KeyboardAndMouse::VK_TAB.0));
    Ok(())
}

#[tauri::command]
fn open_search() -> Result<(), String> {
    #[cfg(windows)]
    tap_win(Some(windows::Win32::UI::Input::KeyboardAndMouse::VK_S.0));
    Ok(())
}

/// Minimise everything, and restore it on a second press, the way the sliver
/// at the end of the shell's taskbar does.
#[tauri::command]
fn show_desktop() -> Result<(), String> {
    #[cfg(windows)]
    tap_win(Some(windows::Win32::UI::Input::KeyboardAndMouse::VK_D.0));
    Ok(())
}

#[tauri::command]
fn volume() -> Option<audio::Volume> {
    #[cfg(windows)]
    {
        audio::volume()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[tauri::command]
fn set_volume(level: u32) -> Option<audio::Volume> {
    #[cfg(windows)]
    {
        audio::set_volume(level)
    }
    #[cfg(not(windows))]
    {
        let _ = level;
        None
    }
}

#[tauri::command]
fn toggle_speakers() -> Option<audio::Volume> {
    #[cfg(windows)]
    {
        audio::toggle_speakers()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Open the Windows language picker, as clicking the shell's indicator does.
#[tauri::command]
fn cycle_layout() -> Result<(), String> {
    #[cfg(windows)]
    tap_win(Some(
        windows::Win32::UI::Input::KeyboardAndMouse::VK_SPACE.0,
    ));
    Ok(())
}

#[tauri::command]
fn keyboard_layout() -> Option<kbd::Layout> {
    #[cfg(windows)]
    {
        kbd::current()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[tauri::command]
fn battery() -> Option<power::Battery> {
    #[cfg(windows)]
    {
        power::battery()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[tauri::command]
fn list_windows(state: tauri::State<'_, BarState>) -> Vec<winlist::WindowInfo> {
    #[cfg(windows)]
    {
        let max = state.config.lock().unwrap().widgets.apps_max;
        let (monitor, skip) = {
            let held = state.monitor.lock().unwrap();
            *held
        };
        winlist::list(monitor, skip, max)
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        Vec::new()
    }
}

#[tauri::command]
fn focus_window(hwnd: isize) {
    #[cfg(windows)]
    winlist::focus(hwnd);
    #[cfg(not(windows))]
    let _ = hwnd;
}

#[tauri::command]
fn close_window(hwnd: isize) {
    #[cfg(windows)]
    winlist::close(hwnd);
    #[cfg(not(windows))]
    let _ = hwnd;
}

/// Open the Windows network pane, which is what a click on the widget wants.
#[tauri::command]
fn open_network_settings() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", "ms-availablenetworks:"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        Err("only on windows".into())
    }
}

#[tauri::command]
fn net_status() -> Option<net::NetInfo> {
    #[cfg(windows)]
    {
        Some(net::status())
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[tauri::command]
fn mic_muted() -> Option<bool> {
    #[cfg(windows)]
    {
        audio::mic_muted()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[tauri::command]
fn toggle_mic() -> Option<bool> {
    #[cfg(windows)]
    {
        audio::toggle_mic()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Push the app list to the bar once a second.
///
/// A plain thread rather than an async task: every call in here is a blocking
/// win32 one, and the only thing shared is the handle, which is `Send`. The
/// list is only emitted when it differs, so an idle desktop costs one
/// enumeration a second and no IPC.
#[cfg(windows)]
fn spawn_app_poll(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last: Vec<winlist::WindowInfo> = Vec::new();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let state = match app.try_state::<BarState>() {
                Some(s) => s,
                None => return,
            };
            // Cheap, and it is the only thing that catches a resolution change
            // or another appbar shoving us off the edge after the fact.
            if let Some(bar) = state.bar.lock().unwrap().as_ref() {
                bar.ensure_placed();
            }
            if !state.config.lock().unwrap().widgets.apps {
                if !last.is_empty() {
                    last.clear();
                    let _ = app.emit("apps-changed", &last);
                }
                continue;
            }
            let max = state.config.lock().unwrap().widgets.apps_max;
            let (monitor, skip) = *state.monitor.lock().unwrap();
            let now = winlist::list(monitor, skip, max);
            let same = now.len() == last.len()
                && now.iter().zip(&last).all(|(a, b)| {
                    a.hwnd == b.hwnd && a.active == b.active && a.title == b.title
                });
            if !same {
                let _ = app.emit("apps-changed", &now);
                last = now;
            }
        }
    });
}

fn main() {
    // Both before anything is hidden: the net has to be up first, and a hide
    // left behind by a crashed run has to be undone before we hide again.
    appbar::install_safety_net();
    appbar::recover_from_crash();

    let cfg = Config::load();

    tauri::Builder::default()
        .manage(BarState {
            bar: Mutex::new(None),
            config: Mutex::new(cfg.clone()),
            #[cfg(windows)]
            monitor: Mutex::new((Default::default(), Default::default())),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            reset_config,
            bar_status,
            open_settings,
            launch_claude,
            mic_muted,
            toggle_mic,
            open_start_menu,
            open_task_view,
            open_search,
            show_desktop,
            volume,
            set_volume,
            toggle_speakers,
            battery,
            keyboard_layout,
            cycle_layout,
            net_status,
            open_network_settings,
            list_windows,
            focus_window,
            close_window
        ])
        .setup(move |app| {
            let window = app
                .get_webview_window("bar")
                .ok_or("the bar window is missing from tauri.conf.json")?;

            #[cfg(windows)]
            {
                let hwnd = windows::Win32::Foundation::HWND(window.hwnd()?.0 as *mut _);
                let bar = AppBar::new(
                    hwnd,
                    edge_from(&cfg.edge),
                    cfg.thickness as i32,
                    cfg.replace_shell,
                )?;
                let state = app.state::<BarState>();
                state.bar.lock().unwrap().replace(bar);
                // Recorded after the bar has been placed, so the monitor is the
                // one it actually landed on.
                *state.monitor.lock().unwrap() = (winlist::monitor_of(hwnd), hwnd);

                spawn_app_poll(app.handle().clone());
            }

            // Shown only once it is sized and placed, so it never flashes at
            // the default geometry.
            window.show()?;

            // Showing the window can put back the geometry tauri.conf.json
            // asked for, which would drop the bar at the top of the screen, so
            // the placement is re-asserted on the other side of it. After this
            // the poll thread keeps it honest.
            #[cfg(windows)]
            if let Some(bar) = app.state::<BarState>().bar.lock().unwrap().as_ref() {
                bar.reposition();
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("codabar failed to start")
        .run(|app, event| {
            // Give the edge and the shell's taskbar back before the process
            // goes, rather than leaving it to whatever runs Drop last.
            if matches!(event, RunEvent::Exit) {
                if let Some(state) = app.try_state::<BarState>() {
                    state.bar.lock().unwrap().take();
                }
            }
        });
}
