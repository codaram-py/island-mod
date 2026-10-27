//! The keyboard-language indicator: the two lines Windows puts in the tray.
//!
//! The top line is the language, the bottom one the layout within it -- "POR"
//! over "PTB2" on a Brazilian ABNT2 keyboard.

use serde::Serialize;
use windows::Win32::Globalization::{
    GetLocaleInfoW, LOCALE_SABBREVLANGNAME, LOCALE_SENGLISHLANGUAGENAME,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardLayoutNameW};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    /// Three letters for the language, e.g. `POR`.
    pub language: String,
    /// The layout within that language, e.g. `PTB2`.
    pub layout: String,
}

fn locale_info(langid: u32, what: u32) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetLocaleInfoW(langid, what, Some(&mut buf)) };
    if n <= 1 {
        return String::new();
    }
    // The count includes the terminator.
    String::from_utf16_lossy(&buf[..(n as usize - 1)])
}

pub fn current() -> Option<Layout> {
    // Thread 0 means the foreground thread's layout, which is the one the user
    // is actually typing with rather than ours.
    let hkl = unsafe { GetKeyboardLayout(0) };
    let langid = (hkl.0 as usize & 0xFFFF) as u32;
    if langid == 0 {
        return None;
    }

    // Windows' own indicator shows the first three letters of the language's
    // English name -- Portuguese becomes POR, Spanish SPA -- not the locale
    // abbreviation, which would be PTB here.
    let english = locale_info(langid, LOCALE_SENGLISHLANGUAGENAME);
    let language = if english.is_empty() {
        String::new()
    } else {
        english.chars().take(3).collect::<String>().to_uppercase()
    };

    let abbrev = locale_info(langid, LOCALE_SABBREVLANGNAME).to_uppercase();

    // The KLID is eight hex digits: the low half is the language, the high
    // half picks between that language's layouts. Zero is the primary one;
    // anything else is a variant, and Windows numbers those from two --
    // 00010416 is ABNT2, shown as PTB2.
    let mut klid = [0u16; 9];
    let variant = if unsafe { GetKeyboardLayoutNameW(&mut klid) }.is_ok() {
        let s = String::from_utf16_lossy(&klid[..8]);
        match u32::from_str_radix(s.get(..4).unwrap_or("0"), 16) {
            Ok(0) | Err(_) => String::new(),
            Ok(n) => (n + 1).to_string(),
        }
    } else {
        String::new()
    };

    if language.is_empty() && abbrev.is_empty() {
        return None;
    }
    Some(Layout {
        language: if language.is_empty() { abbrev.clone() } else { language },
        layout: format!("{abbrev}{variant}"),
    })
}
