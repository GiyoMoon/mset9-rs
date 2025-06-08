use std::{fs::File, path::Path};

use sysinfo::Disks;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use crate::error::MSET9Error;

const REQUIRED_FILES: [(&str, Option<u64>); 6] = [
    ("boot9strap/boot9strap.firm", Some(15872)),
    ("boot9strap/boot9strap.firm.sha", None),
    ("boot.firm", None),
    ("boot.3dsx", None),
    ("b9", None),
    ("SafeB9S.bin", None),
];

pub fn run_checks(sd_root: &Path) -> Result<(), MSET9Error> {
    check_sd_card(sd_root)?;
    check_root(sd_root)?;
    check_write_protection(sd_root)?;
    check_free_space(sd_root, 16 * 1024 * 1024)?;
    let has_missing_files = REQUIRED_FILES
        .iter()
        .any(|(file, size)| !check_file(sd_root, file, *size));
    if has_missing_files {
        return Err(MSET9Error::UserError(
            "One or more files are missing or malformed!\nPlease re-extract the MSET9 zip file, overwriting any existing files when prompted.".to_string(),
            7,
        ));
    }
    Ok(())
}

pub fn check_sd_card(sd_root: &Path) -> Result<(), MSET9Error> {
    #[cfg(not(target_os = "windows"))]
    let script_dev = fs::metadata(sd_root)?.dev();
    #[cfg(not(target_os = "windows"))]
    let boot_dev = fs::metadata("/")?.dev();
    #[cfg(target_os = "windows")]
    let script_dev = get_volume_serial(sd_root.to_str().unwrap())?;
    #[cfg(target_os = "windows")]
    let boot_dev = get_volume_serial("C:\\")?;

    if script_dev == boot_dev {
        return Err(MSET9Error::UserError(
            format!(
                "Script is not running on your SD card! Current location: {}",
                sd_root.to_str().unwrap()
            ),
            1,
        ));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn get_volume_serial(path: &str) -> Result<u32, MSET9Error> {
    use std::{ffi::OsStr, io, mem, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::{HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_READ,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
            OPEN_EXISTING,
        },
    };

    let wide_path: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();

    let handle: HANDLE = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            FILE_GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error().into());
    }

    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { mem::zeroed() };
    let ret = unsafe { GetFileInformationByHandle(handle, &mut info) };

    if ret == 0 {
        return Err(io::Error::last_os_error().into());
    }

    Ok(info.dwVolumeSerialNumber)
}

pub fn check_root(sd_root: &Path) -> Result<(), MSET9Error> {
    if !sd_root.join("Nintendo 3DS").exists() {
        return Err(MSET9Error::UserError(
            "Couldn't find Nintendo 3DS folder! Ensure that you are running this script from the root of the SD card.\nIf that doesn't work, eject the SD card, and put it back in your console. Turn it on and off again, then rerun this script.".to_string(),
            1,
        ));
    }
    Ok(())
}

pub fn check_write_protection(sd_root: &Path) -> Result<(), MSET9Error> {
    let test_file_path = Path::new(sd_root).join(".test_write_access");
    let can_write = match File::create(&test_file_path) {
        Ok(_) => {
            std::fs::remove_file(test_file_path).ok();
            true
        }
        Err(_) => false,
    };

    if !can_write {
        return Err(MSET9Error::UserError(
            "Your SD card is write protected! If using a full size SD card, ensure that the lock switch is facing upwards.\nVisual aid: https://nintendohomebrew.com/assets/img/nhmemes/sdlock.png".to_string(),
            2,
        ));
    }

    Ok(())
}

pub fn check_free_space(path: &Path, required_bytes: u64) -> Result<(), MSET9Error> {
    let disks = Disks::new_with_refreshed_list();

    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    for disk in disks.list() {
        let mount_point = disk
            .mount_point()
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());

        if path == mount_point {
            let available = disk.available_space();
            if available >= required_bytes {
                return Ok(());
            } else {
                return Err(MSET9Error::UserError(
                    format!(
                        "You need at least {}MB free space on your SD card! Please free up some space and try again.",
                        required_bytes / 1024 / 1024
                    ),
                    6,
                ));
            }
        }
    }

    Err(MSET9Error::InternalError(
        "Unable to determine available space on the SD card. Disk not found".to_string(),
    ))
}

pub fn check_file(path: &Path, filename: &str, filesize: Option<u64>) -> bool {
    let filepath = path.join(filename);
    if !filepath.exists() || !filepath.is_file() {
        return false;
    }
    if let Some(size) = filesize {
        if let Ok(metadata) = filepath.metadata() {
            if metadata.len() != size {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}
