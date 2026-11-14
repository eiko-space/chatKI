//! Erkennt, ob ein Laufwerk eine SSD oder eine (langsame) HDD ist – ohne Admin-Rechte.

/// Laufwerksbuchstabe (z. B. 'C') → ist es eine HDD (dreht sich)? `None` = unbekannt/Netzlaufwerk.
#[cfg(windows)]
pub fn is_hdd(drive: char) -> Option<bool> {
    use crate::win::wide;
    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::System::Ioctl::{DEVICE_SEEK_PENALTY_DESCRIPTOR, IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, StorageDeviceSeekPenaltyProperty, STORAGE_PROPERTY_QUERY};
    use windows_sys::Win32::System::IO::DeviceIoControl;

    let path = wide(&format!(r"\\.\{drive}:"));
    // SAFETY: Handle wird geprüft und geschlossen.
    unsafe {
        let h = CreateFileW(path.as_ptr(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut());
        if h == INVALID_HANDLE_VALUE {
            return None;
        }
        let query = STORAGE_PROPERTY_QUERY { PropertyId: StorageDeviceSeekPenaltyProperty, QueryType: PropertyStandardQuery, AdditionalParameters: [0] };
        let mut desc: DEVICE_SEEK_PENALTY_DESCRIPTOR = std::mem::zeroed();
        let mut returned = 0u32;
        let ok = DeviceIoControl(
            h,
            IOCTL_STORAGE_QUERY_PROPERTY,
            (&query as *const STORAGE_PROPERTY_QUERY).cast(),
            std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            (&mut desc as *mut DEVICE_SEEK_PENALTY_DESCRIPTOR).cast(),
            std::mem::size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
            &mut returned,
            std::ptr::null_mut(),
        );
        CloseHandle(h);
        // IncursSeekPenalty != 0 → das Laufwerk hat eine Suchzeit, also eine drehende Festplatte.
        (ok != 0).then_some(desc.IncursSeekPenalty != 0)
    }
}

#[cfg(not(windows))]
pub fn is_hdd(_drive: char) -> Option<bool> {
    None
}
