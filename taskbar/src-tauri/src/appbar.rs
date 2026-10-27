//! Taking the screen edge away from the Windows taskbar, and giving it back.
//!
//! The bar is a real appbar, not an overlay: `SHAppBarMessage` reserves the
//! strip, so a maximized window stops above it the way it stopped above the
//! shell's own taskbar. Nothing here sets topmost.
//!
//! The shell's taskbar is only hidden, never closed, and every path that can
//! end the process puts it back -- including a panic and a crash on the run
//! before this one, which is what the marker file is for. Leaving someone with
//! no taskbar at all is the one failure this module exists to prevent.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE,
    ABM_SETPOS, APPBARDATA,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::SPI_SETWORKAREA;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetSystemMetrics, IsWindow, IsWindowVisible, SendMessageTimeoutW,
    GetWindowRect, SetWindowPos, ShowWindow, SMTO_ABORTIFHUNG, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE,
    SWP_NOSENDCHANGING, SWP_NOZORDER, SW_HIDE, SW_SHOWNA, WM_SETTINGCHANGE,
};

pub use crate::geom::Edge;

use crate::geom::{edge_rect, to_physical, Rect};

impl Edge {
    fn raw(self) -> u32 {
        match self {
            Edge::Top => ABE_TOP,
            Edge::Bottom => ABE_BOTTOM,
            Edge::Left => ABE_LEFT,
            Edge::Right => ABE_RIGHT,
        }
    }
}

fn to_rect(r: Rect) -> RECT {
    RECT { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

fn from_rect(r: RECT) -> Rect {
    Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

/// Set once we have hidden the shell's bar, so the panic hook knows to undo it.
static HID_SHELL: AtomicBool = AtomicBool::new(false);

fn marker_path() -> PathBuf {
    let dir = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    dir.join("codabar").join("shell-hidden.marker")
}

/// The shell's own taskbar windows: the primary one, plus one per extra monitor.
fn shell_bars() -> Vec<HWND> {
    let mut out = Vec::new();
    unsafe {
        if let Ok(h) = FindWindowW(w!("Shell_TrayWnd"), None) {
            if !h.is_invalid() {
                out.push(h);
            }
        }
        // Secondary bars share a class; walk them until FindWindowEx runs dry.
        let mut prev = HWND(std::ptr::null_mut());
        loop {
            match windows::Win32::UI::WindowsAndMessaging::FindWindowExW(
                HWND(std::ptr::null_mut()),
                prev,
                w!("Shell_SecondaryTrayWnd"),
                None,
            ) {
                Ok(h) if !h.is_invalid() => {
                    out.push(h);
                    prev = h;
                }
                _ => break,
            }
        }
    }
    out
}

fn set_shell_visible(visible: bool) {
    let cmd = if visible { SW_SHOWNA } else { SW_HIDE };
    for h in shell_bars() {
        unsafe {
            if IsWindow(h).as_bool() {
                let _ = ShowWindow(h, cmd);
            }
        }
    }
}

/// Undo a hide left behind by a run that crashed before it could restore.
///
/// Called once at startup, before we touch anything.
pub fn recover_from_crash() {
    let marker = marker_path();
    if !marker.exists() {
        return;
    }
    set_shell_visible(true);
    let _ = std::fs::remove_file(&marker);
}

/// Owns the reserved strip. Dropping it gives the edge and the shell bar back.
pub struct AppBar {
    hwnd: HWND,
    edge: Edge,
    thickness: i32,
    registered: bool,
    replace_shell: bool,
}

impl AppBar {
    /// Reserve `thickness` pixels along `edge` for `hwnd` and hide the shell's bar.
    ///
    /// `hwnd` must be a real top-level window; it is the handle Windows will
    /// send appbar notifications to.
    /// `replace_shell` false leaves the shell's taskbar on screen and only
    /// reserves the edge, which is the safe way to try the bar out.
    pub fn new(
        hwnd: HWND,
        edge: Edge,
        thickness: i32,
        replace_shell: bool,
    ) -> windows::core::Result<Self> {
        let mut bar = AppBar {
            hwnd,
            edge,
            thickness: thickness.max(1),
            registered: false,
            replace_shell,
        };

        let mut abd = bar.data();
        let registered = unsafe { SHAppBarMessage(ABM_NEW, &mut abd) } != 0;
        if !registered {
            return Err(windows::core::Error::from_win32());
        }
        bar.registered = true;

        // Hide the shell bar only once we hold the reservation, so there is no
        // window where neither bar owns the edge.
        if replace_shell {
            std::fs::create_dir_all(marker_path().parent().unwrap()).ok();
            std::fs::write(marker_path(), b"1").ok();
            HID_SHELL.store(true, Ordering::SeqCst);
            set_shell_visible(false);
        }

        bar.reposition();
        // A window that was already maximized keeps the work area it was
        // maximized against, so it would sit under the strip we just took --
        // which, on a see-through bar, reads as that window's title bar
        // showing through. Telling everyone the work area moved makes them
        // re-flow.
        broadcast_workarea_change();
        Ok(bar)
    }

    fn data(&self) -> APPBARDATA {
        APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: self.hwnd,
            uCallbackMessage: 0,
            uEdge: self.edge.raw(),
            rc: RECT::default(),
            lParam: LPARAM(0),
        }
    }

    /// `thickness` in physical pixels.
    ///
    /// It is stored and configured in logical pixels, because that is what the
    /// webview lays the bar out in; the win32 side wants physical ones. On a
    /// scaled display the two differ, and mixing them is what leaves the bar
    /// taller or shorter than the content inside it.
    fn physical_thickness(&self) -> i32 {
        to_physical(self.thickness, unsafe { GetDpiForWindow(self.hwnd) })
    }

    /// The rectangle we want, in virtual-screen coordinates.
    ///
    /// Taken from the monitor the bar is on rather than `SM_CXSCREEN`, which
    /// only ever describes the primary one: on a second monitor those metrics
    /// would put the bar on the wrong screen entirely.
    fn desired(&self) -> RECT {
        let screen = unsafe {
            let monitor = MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                from_rect(info.rcMonitor)
            } else {
                Rect {
                    left: 0,
                    top: 0,
                    right: GetSystemMetrics(SM_CXSCREEN),
                    bottom: GetSystemMetrics(SM_CYSCREEN),
                }
            }
        };
        to_rect(edge_rect(screen, self.edge, self.physical_thickness()))
    }

    /// Take the space and move there.
    ///
    /// `ABM_QUERYPOS` asks the system where an appbar may sit and slides the
    /// rectangle clear of the ones already there -- the shell's taskbar among
    /// them, because hiding its window with `ShowWindow` does not retract its
    /// reservation. Honouring that while replacing it parks us one taskbar
    /// height up the screen, with a strip of wallpaper underneath. So when we
    /// are the taskbar we keep our own rectangle and claim the edge outright,
    /// and only negotiate when we are sharing the screen with the shell's bar.
    pub fn reposition(&self) {
        if !self.registered {
            return;
        }
        let want = self.desired();
        let mut abd = self.data();
        abd.rc = want;

        if !self.replace_shell {
            unsafe { SHAppBarMessage(ABM_QUERYPOS, &mut abd) };
            let t = self.physical_thickness();
            match self.edge {
                Edge::Top => abd.rc.bottom = abd.rc.top + t,
                Edge::Bottom => abd.rc.top = abd.rc.bottom - t,
                Edge::Left => abd.rc.right = abd.rc.left + t,
                Edge::Right => abd.rc.left = abd.rc.right - t,
            }
        }

        // SETPOS writes the approved rectangle back into abd.rc, and that
        // answer is subject to the same reservations QUERYPOS is -- the hidden
        // shell taskbar's among them, which can shrink ours to nothing. So
        // while we are replacing that taskbar the window goes where we asked,
        // not where the answer came back.
        unsafe { SHAppBarMessage(ABM_SETPOS, &mut abd) };
        let placed = if self.replace_shell { want } else { abd.rc };

        let (x, y) = (placed.left, placed.top);
        let (cx, cy) = (placed.right - placed.left, placed.bottom - placed.top);
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND::default(),
                x,
                y,
                cx,
                cy,
                SWP_NOACTIVATE | SWP_NOSENDCHANGING | SWP_NOZORDER,
            );
        }
    }

    /// Put the window back if something has moved or resized it.
    ///
    /// A display change, a resolution change or another appbar registering can
    /// all shift us after the fact, and there is no notification for it
    /// without a callback message on the window. Comparing against the
    /// rectangle we want is a `GetWindowRect` -- cheap enough to do on the
    /// same beat as everything else, and it only touches the window when it
    /// has actually drifted.
    pub fn ensure_placed(&self) {
        if !self.registered {
            return;
        }
        let wanted = self.desired();
        let mut now = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut now) }.is_err() {
            return;
        }
        if now != wanted {
            self.reposition();
        }
    }

    /// Change how thick the strip is and re-reserve.
    pub fn set_thickness(&mut self, thickness: i32) {
        self.thickness = thickness.max(1);
        self.reposition();
    }

    pub fn edge(&self) -> Edge {
        self.edge
    }

    /// Move to another edge, keeping the reservation.
    pub fn set_edge(&mut self, edge: Edge) {
        if edge == self.edge {
            return;
        }
        self.edge = edge;
        self.reposition();
    }

    /// True while the bar is laid out along the long axis of the screen.
    pub fn horizontal(&self) -> bool {
        self.edge.horizontal()
    }

    /// Put the shell's bar back and release the edge. Idempotent.
    fn teardown(&mut self) {
        if self.registered {
            let mut abd = self.data();
            unsafe { SHAppBarMessage(ABM_REMOVE, &mut abd) };
            self.registered = false;
        }
        if HID_SHELL.swap(false, Ordering::SeqCst) {
            set_shell_visible(true);
            let _ = std::fs::remove_file(marker_path());
            // Nudge the shell so it re-reads the work area it just got back.
            broadcast_workarea_change();
        }
    }
}

impl Drop for AppBar {
    fn drop(&mut self) {
        self.teardown();
    }
}

/// Tell every top-level window that the work area moved.
///
/// `SPI_SETWORKAREA` is the hint that makes a maximized window re-fit itself;
/// sent with a timeout so one hung app cannot stall the bar coming up.
fn broadcast_workarea_change() {
    unsafe {
        let mut res: usize = 0;
        let _ = SendMessageTimeoutW(
            windows::Win32::UI::WindowsAndMessaging::HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(SPI_SETWORKAREA.0 as usize),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            1000,
            Some(&mut res as *mut usize as *mut _),
        );
    }
}

/// Restore the shell's taskbar from a panic or a signal, where no `Drop` runs.
///
/// Install once, before the first `AppBar::new`.
pub fn install_safety_net() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if HID_SHELL.swap(false, Ordering::SeqCst) {
            set_shell_visible(true);
            let _ = std::fs::remove_file(marker_path());
        }
        previous(info);
    }));
}

/// Whether the shell's taskbar is currently on screen.
pub fn shell_visible() -> bool {
    shell_bars()
        .first()
        .map(|h| unsafe { IsWindowVisible(*h).as_bool() })
        .unwrap_or(false)
}
