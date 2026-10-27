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
    eCapture, eCommunications, eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
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

/// The speakers, as opposed to the microphone above.
///
/// eMultimedia rather than eCommunications: this is the device the volume
/// slider in the shell's own flyout moves.
fn speakers() -> windows::core::Result<IAudioEndpointVolume> {
    ensure_com();
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
    }
}

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq)]
pub struct Volume {
    /// 0-100, as the shell shows it.
    pub level: u32,
    pub muted: bool,
}

pub fn volume() -> Option<Volume> {
    let ep = speakers().ok()?;
    unsafe {
        // The scalar is the linear 0..1 the slider moves, not the dB scale.
        let level = ep.GetMasterVolumeLevelScalar().ok()?;
        let muted = ep.GetMute().ok()?.as_bool();
        Some(Volume {
            level: (level * 100.0).round().clamp(0.0, 100.0) as u32,
            muted,
        })
    }
}

pub fn set_volume(level: u32) -> Option<Volume> {
    let ep = speakers().ok()?;
    unsafe {
        ep.SetMasterVolumeLevelScalar(level.min(100) as f32 / 100.0, &GUID::zeroed())
            .ok()?;
    }
    volume()
}

pub fn toggle_speakers() -> Option<Volume> {
    let ep = speakers().ok()?;
    unsafe {
        let now = ep.GetMute().ok()?.as_bool();
        ep.SetMute(!now, &GUID::zeroed()).ok()?;
    }
    volume()
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
