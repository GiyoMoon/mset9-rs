use std::{
    fs::{self, File},
    path::Path,
};

use sysinfo::Disks;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

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
    let script_dev = fs::metadata(sd_root)?.dev();
    #[cfg(unix)]
    let boot_dev = fs::metadata("/")?.dev();
    #[cfg(windows)]
    let boot_dev = fs::metadata("C:\\")?.dev();

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
        let mount_point = disk.mount_point();

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
