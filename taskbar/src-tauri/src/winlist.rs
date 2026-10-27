//! The app list: the top-level windows on the bar's monitor, their icons, and
//! the two things you can do to one.
//!
//! Enumeration is cheap enough to run every second; pulling an icon out of a
//! window is not, so icons are cached per window and only re-read when the
//! process behind it changes.

use std::collections::HashMap;
use std::sync::Mutex;

use base64::Engine;
use serde::Serialize;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, MAX_PATH, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, MonitorFromWindow, ReleaseDC, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ, HMONITOR,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, EnumWindows, GetClassLongPtrW, GetForegroundWindow, GetIconInfo, GetWindow,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, PostMessageW, SendMessageTimeoutW, SetForegroundWindow, ShowWindow,
    GCLP_HICON, GCLP_HICONSM, GWL_EXSTYLE, GW_OWNER, HICON, ICONINFO, ICON_BIG, ICON_SMALL2,
    SMTO_ABORTIFHUNG, SW_RESTORE, WM_CLOSE, WM_GETICON, WS_EX_TOOLWINDOW,
};

pub use windows::Win32::Foundation::HWND as BarHwnd;
pub use windows::Win32::Graphics::Gdi::HMONITOR as BarMonitor;

#[derive(Serialize, Clone, Debug)]
pub struct WindowInfo {
    /// The raw `HWND`, round-tripped through the frontend so a click can name
    /// the window it meant.
    pub hwnd: isize,
    pub title: String,
    /// `data:image/png;base64,...`, or empty when no icon could be read.
    pub icon: String,
    pub active: bool,
}

static ICONS: Mutex<Option<HashMap<isize, (String, String)>>> = Mutex::new(None);

// ---- enumeration -----------------------------------------------------------

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam.0 as *mut Vec<HWND>);
    out.push(hwnd);
    true.into()
}

/// The filter the shell's own taskbar applies, near enough: visible, titled,
/// not owned by another window, not a tool window, and not cloaked.
///
/// Cloaking is the one that matters on Windows 10 and up -- that is how the
/// shell hides UWP apps that are suspended and the phantom `ApplicationFrame`
/// windows that would otherwise show up as duplicates.
unsafe fn is_app_window(hwnd: HWND) -> bool {
    if !IsWindowVisible(hwnd).as_bool() {
        return false;
    }
    if GetWindowTextLengthW(hwnd) == 0 {
        return false;
    }
    if GetWindow(hwnd, GW_OWNER).map(|h| !h.is_invalid()).unwrap_or(false) {
        return false;
    }
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if ex & WS_EX_TOOLWINDOW.0 != 0 {
        return false;
    }
    let mut cloaked: u32 = 0;
    let ok = DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut u32 as *mut _,
        std::mem::size_of::<u32>() as u32,
    );
    if ok.is_ok() && cloaked != 0 {
        return false;
    }
    true
}

unsafe fn title_of(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(hwnd, &mut buf);
    String::from_utf16_lossy(&buf[..n as usize])
}

unsafe fn exe_of(hwnd: HWND) -> String {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return String::new();
    }
    // LIMITED_INFORMATION is the one that works without elevation against
    // processes at the same integrity level.
    let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
        return String::new();
    };
    let mut buf = vec![0u16; MAX_PATH as usize];
    let mut len = buf.len() as u32;
    let got = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0),
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut len,
    );
    let _ = windows::Win32::Foundation::CloseHandle(handle);
    if got.is_err() {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

// ---- icons -----------------------------------------------------------------

/// Ask the window for its icon, then its class, then the executable behind it.
///
/// `WM_GETICON` goes through `SendMessageTimeout`: a hung app must not hang the
/// bar with it.
unsafe fn icon_handle(hwnd: HWND, exe: &str) -> Option<(HICON, bool)> {
    for kind in [ICON_SMALL2, ICON_BIG] {
        let mut res: usize = 0;
        let _ = SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            WPARAM(kind as usize),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            120,
            Some(&mut res as *mut usize as *mut _),
        );
        if res != 0 {
            return Some((HICON(res as *mut _), false));
        }
    }
    for which in [GCLP_HICONSM, GCLP_HICON] {
        let h = GetClassLongPtrW(hwnd, which);
        if h != 0 {
            return Some((HICON(h as *mut _), false));
        }
    }
    if !exe.is_empty() {
        let wide: Vec<u16> = exe.encode_utf16().chain(std::iter::once(0)).collect();
        let mut small = [HICON::default(); 1];
        let n = windows::Win32::UI::Shell::ExtractIconExW(
            windows::core::PCWSTR(wide.as_ptr()),
            0,
            None,
            Some(small.as_mut_ptr()),
            1,
        );
        if n > 0 && !small[0].is_invalid() {
            // Ours to destroy, unlike the two above which belong to the window.
            return Some((small[0], true));
        }
    }
    None
}

/// Copy an icon's pixels out as a PNG data URL.
unsafe fn icon_to_data_url(hicon: HICON) -> Option<String> {
    let mut info = ICONINFO::default();
    GetIconInfo(hicon, &mut info).ok()?;

    let colour = info.hbmColor;
    let mask = info.hbmMask;
    let cleanup = |b: HBITMAP| {
        if !b.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(b.0));
        }
    };

    let mut bm = BITMAP::default();
    let got = GetObjectW(
        HGDIOBJ(colour.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut _ as *mut _),
    );
    if got == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 || bm.bmWidth > 512 || bm.bmHeight > 512 {
        cleanup(colour);
        cleanup(mask);
        return None;
    }
    let (w, h) = (bm.bmWidth, bm.bmHeight);

    let mut header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        // Negative height asks for a top-down buffer, which is the order PNG
        // wants, so there is no row flip afterwards.
        biHeight: -h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut bi = BITMAPINFO {
        bmiHeader: header,
        ..Default::default()
    };

    let mut buf = vec![0u8; (w * h * 4) as usize];
    let hdc = GetDC(None);
    let lines = GetDIBits(
        hdc,
        colour,
        0,
        h as u32,
        Some(buf.as_mut_ptr() as *mut _),
        &mut bi,
        DIB_RGB_COLORS,
    );
    if lines == 0 {
        ReleaseDC(None, hdc);
        cleanup(colour);
        cleanup(mask);
        return None;
    }

    // BGRA from GDI, RGBA for PNG.
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2);
    }

    // A 24-bit icon comes back with every alpha byte zero, which would encode
    // as fully transparent. The AND mask carries the real shape for those.
    if buf.chunks_exact(4).all(|px| px[3] == 0) {
        let mut m = vec![0u8; (w * h * 4) as usize];
        header.biBitCount = 32;
        let mut mi = BITMAPINFO {
            bmiHeader: header,
            ..Default::default()
        };
        let got_mask = GetDIBits(
            hdc,
            mask,
            0,
            h as u32,
            Some(m.as_mut_ptr() as *mut _),
            &mut mi,
            DIB_RGB_COLORS,
        );
        for (i, px) in buf.chunks_exact_mut(4).enumerate() {
            // In the mask, white means "leave the background alone", so those
            // pixels are the transparent ones.
            px[3] = if got_mask != 0 && m[i * 4] != 0 { 0 } else { 255 };
        }
    }
    ReleaseDC(None, hdc);
    cleanup(colour);
    cleanup(mask);

    let mut png = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(&buf).ok()?;
    }
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&png)
    ))
}

/// Cached by window; re-read when the executable behind it changes, which is
/// how a recycled `HWND` is caught.
fn icon_for(hwnd: HWND, exe: &str) -> String {
    let key = hwnd.0 as isize;
    let mut guard = ICONS.lock().unwrap();
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some((cached_exe, data)) = cache.get(&key) {
        if cached_exe == exe {
            return data.clone();
        }
    }
    let data = unsafe {
        match icon_handle(hwnd, exe) {
            Some((hicon, owned)) => {
                let out = icon_to_data_url(hicon).unwrap_or_default();
                if owned {
                    let _ = DestroyIcon(hicon);
                }
                out
            }
            None => String::new(),
        }
    };
    cache.insert(key, (exe.to_string(), data.clone()));
    data
}

// ---- the list --------------------------------------------------------------

/// Every app window on `monitor`, in z-order as `EnumWindows` gives it.
///
/// `skip` is the bar's own window.
pub fn list(monitor: HMONITOR, skip: HWND, max: usize) -> Vec<WindowInfo> {
    let mut handles: Vec<HWND> = Vec::with_capacity(64);
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut handles as *mut _ as isize));
    }

    let foreground = unsafe { GetForegroundWindow() };
    let mut live = Vec::new();
    let mut out = Vec::new();

    for hwnd in handles {
        if hwnd == skip {
            continue;
        }
        unsafe {
            if !is_app_window(hwnd) {
                continue;
            }
            if !monitor.is_invalid()
                && MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) != monitor
            {
                continue;
            }
            let exe = exe_of(hwnd);
            live.push(hwnd.0 as isize);
            if out.len() < max {
                out.push(WindowInfo {
                    hwnd: hwnd.0 as isize,
                    title: title_of(hwnd),
                    icon: icon_for(hwnd, &exe),
                    active: hwnd == foreground,
                });
            }
        }
    }

    // Windows that have gone would otherwise keep their icon alive forever.
    if let Some(cache) = ICONS.lock().unwrap().as_mut() {
        cache.retain(|k, _| live.contains(k));
    }
    out
}

/// The monitor a window is on, for scoping the list to the bar's screen.
pub fn monitor_of(hwnd: HWND) -> HMONITOR {
    unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) }
}

pub fn focus(hwnd: isize) {
    let hwnd = HWND(hwnd as *mut _);
    unsafe {
        // The bar never takes focus, so it is not the foreground process and
        // SetForegroundWindow would be refused; this hands our claim over.
        let _ = windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(
            windows::Win32::UI::WindowsAndMessaging::ASFW_ANY,
        );
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Asks the window to close, the way its own close button does: posted, not
/// sent, so an app that puts up a "save changes?" prompt cannot block the bar.
pub fn close(hwnd: isize) {
    unsafe {
        let _ = PostMessageW(HWND(hwnd as *mut _), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}
