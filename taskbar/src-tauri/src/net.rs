//! The network widget: what we are connected through, and how well.
//!
//! The WLAN side answers for wireless; when there is no wireless interface, or
//! it is down, the network list manager still knows whether anything is
//! connected at all, which is what a machine on ethernet needs.

use serde::Serialize;
use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
use windows::Win32::NetworkManagement::WiFi::{
    wlan_interface_state_connected, wlan_intf_opcode_current_connection, WlanCloseHandle,
    WlanEnumInterfaces, WlanFreeMemory, WlanOpenHandle, WlanQueryInterface,
    WLAN_CONNECTION_ATTRIBUTES, WLAN_INTERFACE_INFO_LIST, WLAN_OPCODE_VALUE_TYPE,
};
use windows::Win32::Networking::NetworkListManager::{
    INetworkListManager, NetworkListManager, NLM_CONNECTIVITY_IPV4_INTERNET,
    NLM_CONNECTIVITY_IPV6_INTERNET,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetInfo {
    /// `wifi`, `wired` or `none`.
    pub kind: String,
    /// The network's name when we have one; empty on a wired link.
    pub ssid: String,
    /// Signal quality, 0-100. Zero on anything but wi-fi.
    pub quality: u32,
    /// Whether the link actually reaches the internet, not just a router.
    pub online: bool,
}

/// Wireless state from the WLAN service.
///
/// `None` means no wireless interface is connected -- either the machine has
/// no radio, or it is off, or it is associating. The caller falls back to the
/// wired answer in every one of those cases.
fn wifi() -> Option<(String, u32)> {
    unsafe {
        let mut handle = HANDLE::default();
        let mut negotiated = 0u32;
        // Version 2 is what every supported Windows speaks.
        if WlanOpenHandle(2, None, &mut negotiated, &mut handle) != ERROR_SUCCESS.0 {
            return None;
        }

        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if WlanEnumInterfaces(handle, None, &mut list) != ERROR_SUCCESS.0 || list.is_null() {
            let _ = WlanCloseHandle(handle, None);
            return None;
        }

        let mut found = None;
        let count = (*list).dwNumberOfItems as usize;
        for i in 0..count {
            let info = &(*list).InterfaceInfo.as_ptr().add(i).read();
            if info.isState != wlan_interface_state_connected {
                continue;
            }
            let mut size = 0u32;
            let mut data: *mut std::ffi::c_void = std::ptr::null_mut();
            let mut kind = WLAN_OPCODE_VALUE_TYPE::default();
            let rc = WlanQueryInterface(
                handle,
                &info.InterfaceGuid,
                wlan_intf_opcode_current_connection,
                None,
                &mut size,
                &mut data,
                Some(&mut kind),
            );
            if rc != ERROR_SUCCESS.0 || data.is_null() {
                continue;
            }
            let attrs = &*(data as *const WLAN_CONNECTION_ATTRIBUTES);
            let ssid = &attrs.wlanAssociationAttributes.dot11Ssid;
            let len = (ssid.uSSIDLength as usize).min(ssid.ucSSID.len());
            // The SSID is bytes, not UTF-16, and is not required to be UTF-8
            // either -- lossy is the only honest reading.
            let name = String::from_utf8_lossy(&ssid.ucSSID[..len]).into_owned();
            let quality = attrs.wlanAssociationAttributes.wlanSignalQuality.min(100);
            WlanFreeMemory(data);
            found = Some((name, quality));
            break;
        }

        WlanFreeMemory(list as *mut _);
        let _ = WlanCloseHandle(handle, None);
        found
    }
}

/// Whether anything on this machine reaches the internet.
fn online() -> bool {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(nlm) = CoCreateInstance::<_, INetworkListManager>(&NetworkListManager, None, CLSCTX_ALL)
        else {
            return false;
        };
        match nlm.GetConnectivity() {
            Ok(c) => {
                (c.0 & NLM_CONNECTIVITY_IPV4_INTERNET.0) != 0
                    || (c.0 & NLM_CONNECTIVITY_IPV6_INTERNET.0) != 0
            }
            Err(_) => false,
        }
    }
}

pub fn status() -> NetInfo {
    let reachable = online();
    match wifi() {
        Some((ssid, quality)) => NetInfo {
            kind: "wifi".into(),
            ssid,
            quality,
            online: reachable,
        },
        // No wireless link: if anything is connected at all it is wired, and
        // if nothing is, say so rather than claiming a dead wi-fi.
        None => NetInfo {
            kind: if reachable { "wired" } else { "none" }.into(),
            ssid: String::new(),
            quality: 0,
            online: reachable,
        },
    }
}
