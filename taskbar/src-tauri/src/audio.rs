//! The microphone widget's data: is the default capture device muted, and mute it.
//!
//! Core Audio is apartment-threaded, so every entry point here initialises COM
//! on its own thread and leaves it that way; `CoInitializeEx` returning
//! `RPC_E_CHANGED_MODE` means the thread already belongs to another apartment,
//! which is fine as long as we do not uninitialise it out from under whoever
//! set it up.

use windows::core::GUID;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

/// Join an apartment if this thread is not in one yet.
///
/// Deliberately does not pair with `CoUninitialize`: these calls run on Tauri's
/// command threads, which outlive any single call, and tearing the apartment
/// down would break the next one.
fn ensure_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

fn endpoint() -> windows::core::Result<IAudioEndpointVolume> {
    ensure_com();
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        // eCommunications is the one Windows' own mic indicator follows.
        let device = enumerator.GetDefaultAudioEndpoint(eCapture, eCommunications)?;
        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
    }
}

/// `None` when there is no capture device at all, which is not an error.
pub fn mic_muted() -> Option<bool> {
    let ep = endpoint().ok()?;
    unsafe { ep.GetMute().ok().map(|m| m.as_bool()) }
}

pub fn set_mic_muted(muted: bool) -> windows::core::Result<()> {
    let ep = endpoint()?;
    unsafe { ep.SetMute(muted, &GUID::zeroed()) }
}

/// Returns the state it landed on, so the caller does not have to read back.
pub fn toggle_mic() -> Option<bool> {
    let now = mic_muted()?;
    set_mic_muted(!now).ok()?;
    Some(!now)
}
