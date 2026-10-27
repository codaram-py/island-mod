//! Battery, for the machines that have one.

use serde::Serialize;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    /// 0-100.
    pub percent: u32,
    pub charging: bool,
    /// Seconds of charge left, when Windows will say; it often will not.
    pub seconds_left: Option<u32>,
}

/// `None` on a desktop, or whenever Windows cannot say -- which is the same
/// answer as far as the widget is concerned: do not draw one.
pub fn battery() -> Option<Battery> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;

    // 255 is the documented "unknown" for both of these.
    if status.BatteryLifePercent == 255 || status.BatteryFlag == 128 {
        return None;
    }
    Some(Battery {
        percent: status.BatteryLifePercent as u32,
        // ACLineStatus is 1 on mains; the flag's 0x08 bit is the charging one.
        charging: status.ACLineStatus == 1 || status.BatteryFlag & 0x08 != 0,
        seconds_left: match status.BatteryLifeTime {
            u32::MAX => None,
            s => Some(s),
        },
    })
}
