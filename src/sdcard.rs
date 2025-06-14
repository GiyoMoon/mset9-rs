use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

#[cfg(target_os = "macos")]
use crate::term::info;
#[cfg(target_os = "macos")]
use fatfs::{FileSystem, FsOptions};
#[cfg(target_os = "macos")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "macos")]
use std::process::Command;

use crate::error::MSET9Error;

pub struct DirEntry {
    pub file_name: String,
    pub is_dir: bool,
}

pub struct SdCard {
    #[cfg(target_os = "macos")]
    fs: fatfs::FileSystem<std::fs::File>,
    #[cfg(target_os = "macos")]
    disk: String,
    #[cfg(not(target_os = "macos"))]
    sd_root: String,
}

#[cfg(target_os = "macos")]
impl SdCard {
    pub fn setup(sd_root: String) -> Result<Self, MSET9Error> {
        let sd_root = Path::new(&sd_root);
        let script_dev = fs::metadata(sd_root).unwrap().dev();

        let device = fs::read_dir("/dev").unwrap().find_map(|disk| {
            let disk = disk.unwrap();
            let diskname = disk.file_name();
            if !diskname.to_str().unwrap().starts_with("disk") {
                return None;
            }
            let disk_path = Path::new("/dev").join(diskname);
            let disk_dev = fs::metadata(&disk_path).unwrap().rdev();
            if disk_dev == script_dev {
                Some(disk_path)
            } else {
                None
            }
        });

        let Some(device) = device else {
            return Err(MSET9Error::InternalError(
            "Couldn't find the disk image of the SD card! Ensure that you are running this script from the root of the SD card.".to_string(),
        ));
        };

        Command::new("diskutil")
        .arg("unmountDisk")
        .arg(device.to_str().unwrap())
        .output()
        .map_err(|_| MSET9Error::UserError("Unable to unmount SD card.\nPlease ensure there's no other app using your SD card.".to_string(), 16))?;

        let img_file = File::options().read(true).write(true).open(&device)?;
        let fs = FileSystem::new(img_file, FsOptions::new()).unwrap();

        Ok(Self {
            fs,
            disk: device.to_str().unwrap().to_string(),
        })
    }

    pub fn read_dir(&self, path: &str) -> Result<impl Iterator<Item = DirEntry>, MSET9Error> {
        let dir = self.fs.root_dir().open_dir(path)?;

        let entries = dir.iter().map(|entry| {
            entry
                .map(|e| DirEntry {
                    file_name: e.file_name().to_string(),
                    is_dir: e.is_dir(),
                })
                .unwrap()
        });

        Ok(entries)
    }

    pub fn rename(&self, src: &str, dst: &str) -> Result<(), MSET9Error> {
        let root = self.fs.root_dir();
        root.rename(src, &root, dst)?;
        Ok(())
    }

    pub fn create_dir(&self, path: &str) -> Result<(), MSET9Error> {
        self.fs.root_dir().create_dir(path)?;
        Ok(())
    }

    pub fn create_file(&self, path: &str, content: Option<&str>) -> Result<(), MSET9Error> {
        let mut file = self.fs.root_dir().create_file(path)?;
        if let Some(content) = content {
            file.truncate()?;
            file.write_all(content.as_bytes())?
        }
        Ok(())
    }

    pub fn remove(&self, path: &str) -> Result<(), MSET9Error> {
        self.fs.root_dir().remove(path)?;
        Ok(())
    }

    // TODO: Doesn't feel as nice, isn't there a better way with fatfs?
    pub fn get_file_size(&self, path: &str) -> Result<u64, MSET9Error> {
        let dir_path = &path[..path.rfind('/').unwrap_or(0)];
        let file_name = path.split('/').next_back().unwrap();
        let dir = self.fs.root_dir().open_dir(dir_path)?;

        let file = dir
            .iter()
            .find(|entry| {
                entry
                    .as_ref()
                    .map(|e| e.file_name() == file_name)
                    .unwrap_or(false)
            })
            .ok_or(MSET9Error::InternalError(format!(
                "File '{file_name}' not found in directory '{dir_path}'",
            )))??;
        Ok(file.len())
    }

    pub fn remove_tree(&self, path: &str) -> Result<(), MSET9Error> {
        let dir = self.fs.root_dir().open_dir(path)?;
        for entry in dir.iter() {
            let entry = entry.unwrap();
            let entryname = entry.file_name();
            if entryname == "." || entryname == ".." {
                continue;
            }
            let full_path = format!("{}/{}", path, entry.file_name());
            if entry.is_dir() {
                self.remove_tree(&full_path)?;
            } else {
                self.remove(&full_path)?;
            }
        }
        self.remove(path)?;
        Ok(())
    }

    pub fn file_exists(&self, path: &str) -> Result<bool, MSET9Error> {
        match self.fs.root_dir().open_file(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    pub fn dir_exists(&self, path: &str) -> Result<bool, MSET9Error> {
        match self.fs.root_dir().open_dir(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    pub fn cleanup(&self) -> Result<(), MSET9Error> {
        info("Remounting SD card...")?;
        Command::new("diskutil")
            .arg("mountDisk")
            .arg(&self.disk)
            .output()
            .map_err(|_| MSET9Error::InternalError("Unable to remount SD card.".to_string()))?;
        Ok(())
    }
}

// #[cfg(windows)]
// let boot_dev = fs::metadata("C:\\")?.dev();

#[cfg(not(target_os = "macos"))]
impl SdCard {
    fn root_path(&self) -> &Path {
        Path::new(&self.sd_root)
    }

    pub fn setup(sd_root: String) -> Result<Self, MSET9Error> {
        Ok(SdCard { sd_root })
    }

    pub fn read_dir(&self, path: &str) -> Result<impl Iterator<Item = DirEntry>, MSET9Error> {
        let full_path = self.root_path().join(path);
        let entries = fs::read_dir(full_path)?.map(|entry| {
            let entry = entry.unwrap();
            DirEntry {
                file_name: entry.file_name().to_string_lossy().into_owned(),
                is_dir: entry.file_type().unwrap().is_dir(),
            }
        });
        Ok(entries)
    }

    pub fn rename(&self, src: &str, dst: &str) -> Result<(), MSET9Error> {
        let src_path = self.root_path().join(src);
        let dst_path = self.root_path().join(dst);
        fs::rename(src_path, dst_path)?;
        Ok(())
    }

    pub fn create_dir(&self, path: &str) -> Result<(), MSET9Error> {
        let full_path = self.root_path().join(path);
        fs::create_dir_all(full_path)?;
        Ok(())
    }

    pub fn create_file(&self, path: &str, content: Option<&str>) -> Result<(), MSET9Error> {
        let full_path = self.root_path().join(path);
        let mut file = File::create(full_path)?;
        if let Some(content) = content {
            file.write_all(content.as_bytes())?;
        }
        Ok(())
    }

    pub fn remove(&self, path: &str) -> Result<(), MSET9Error> {
        let full_path = self.root_path().join(path);
        fs::remove_file(full_path)?;
        Ok(())
    }

    pub fn get_file_size(&self, path: &str) -> Result<u64, MSET9Error> {
        let full_path = self.root_path().join(path);
        let metadata = fs::metadata(full_path)?;
        Ok(metadata.len())
    }

    pub fn remove_tree(&self, path: &str) -> Result<(), MSET9Error> {
        let full_path = self.root_path().join(path);
        fs::remove_dir_all(full_path)?;
        Ok(())
    }

    pub fn file_exists(&self, path: &str) -> Result<bool, MSET9Error> {
        let full_path = self.root_path().join(path);
        Ok(full_path.exists() && full_path.is_file())
    }

    pub fn dir_exists(&self, path: &str) -> Result<bool, MSET9Error> {
        let full_path = self.root_path().join(path);
        Ok(full_path.exists() && full_path.is_dir())
    }

    pub fn cleanup(&self) -> Result<(), MSET9Error> {
        Ok(())
    }
}
