#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appbar;
#[cfg(windows)]
mod audio;
mod config;

use std::sync::Mutex;

use appbar::{AppBar, Edge};
use config::Config;
use tauri::{Manager, RunEvent};

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

/// Open the Windows Start menu by simulating a Win key press.
///
/// `keybd_event` is the simplest route; `SendInput` is the modern one, but for
/// a single modifier tap either works and this needs no extra plumbing.
#[tauri::command]
fn open_start_menu() -> Result<(), String> {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            keybd_event, KEYEVENTF_KEYUP, VK_LWIN,
        };
        keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
        keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }
    Ok(())
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
            open_start_menu
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
                app.state::<BarState>().bar.lock().unwrap().replace(bar);
            }

            // Shown only once it is sized and placed, so it never flashes at
            // the default geometry.
            window.show()?;
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
