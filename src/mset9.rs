use std::fmt::Display;

use console::{Term, style};
use dialoguer::Input;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    console::Console,
    error::MSET9Error,
    sdcard::SdCard,
    term::{self, action_promt, ask_for_console, error, header, info, report_sanity},
};

const ID1_BACKUP_SUFFIX: &str = "_user-id1";

const TRIGGER_FILE: &str = "002F003A.txt";

#[derive(PartialEq, Eq, PartialOrd)]
pub enum HaxState {
    NotCreated,
    NotReady,
    Ready,
    Injected,
    Removed,
}

impl Display for HaxState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HaxState::NotCreated => write!(f, "ID1 not created"),
            HaxState::NotReady => write!(f, "Not ready - check MSET9 status for more details"),
            HaxState::Ready => write!(f, "Ready"),
            HaxState::Injected => write!(f, "Injected"),
            HaxState::Removed => write!(f, "Removed trigger file"),
        }
    }
}

pub fn launch(sd_card: &SdCard) -> Result<(), MSET9Error> {
    term::header()?;

    let mut id0_count = 0;
    let mut id0 = None;
    for entry in sd_card.read_dir("Nintendo 3DS")? {
        if !entry.is_dir {
            continue;
        }
        if !is_3ds_id(&entry.file_name) {
            continue;
        }
        id0_count += 1;
        id0 = Some(entry.file_name);
    }

    let Some(id0) = id0 else {
        return Err(MSET9Error::UserError(
            "Couldn't find ID0 folder! Ensure that you are running this script from the root of the SD card.\nIf that doesn't work, eject the SD card, and put it back in your console. Turn it on and off again, then rerun this script.".to_string(),
            3,
        ));
    };
    if id0_count != 1 {
        return Err(MSET9Error::UserError(
            format!(
                "You don't have 1 ID0 in your Nintendo 3DS folder, you have {id0_count}!\nConsult: https://3ds.hacks.guide/troubleshooting#installing-boot9strap-mset9 for help!"
            ),
            4,
        ));
    }

    let mut console = ask_for_console()?;

    let mut hax_state = HaxState::NotCreated;

    let mut id1_count = 0;
    let mut id1 = None;

    let mut title_dbs_exist = false;
    let mut homemenu_extdata_exists = false;
    let mut miimaker_extdata_exists = false;

    for entry in sd_card.read_dir(&format!("Nintendo 3DS/{id0}"))? {
        if !entry.is_dir {
            info(&format!("Found file in ID0 folder? {}", entry.file_name))?;
            continue;
        }
        let dirname = &entry.file_name;
        if is_3ds_id(dirname)
            || (is_3ds_id(dirname.get(..32).unwrap_or(""))
                && dirname.get(32..).unwrap_or("") == ID1_BACKUP_SUFFIX)
        {
            id1_count += 1;
            id1 = Some(dirname.to_string());
        } else if dirname.contains("sdmc") && dirname.graphemes(true).count() == 32 {
            // endcoded ID1 folder
            let utf16le_bytes: Vec<u8> = dirname
                .encode_utf16()
                .flat_map(|u| u.to_le_bytes())
                .collect();
            let current_hax_id1 = hex::encode(utf16le_bytes);

            let current_console = Console::new_from_encoded_id1(&current_hax_id1);

            if current_console.is_none() {
                info("Found unrecognized/duplicate hacked ID1 in ID0 folder, removing!")?;
                sd_card.remove_tree(&format!("Nintendo 3DS/{id0}/{dirname}"))?;
                continue;
            } else if let Some(current_console) = current_console
                && current_console != console
            {
                info("")?;
                info("")?;
                info("Don't change console model/version in the middle of MSET9!")?;
                info(&format!(
                    "Earlier, you selected: {}",
                    style(&current_console).green()
                ))?;
                info(&format!("Now, you selected: {}", style(&console).green()))?;
                info("Please re-enter the number for your console model and version.")?;
                console = ask_for_console()?;
                if console != current_console {
                    info("Renaming current hacked ID1 to match the new console model/version...")?;
                    let current_hax_id1 = current_console.encoded_id1_readable();
                    let current_hax_id1_path = format!("Nintendo 3DS/{id0}/{current_hax_id1}");
                    let new_hacked_id1 = console.encoded_id1_readable();
                    let new_hacked_id1_path = format!("Nintendo 3DS/{id0}/{new_hacked_id1}");
                    sd_card.rename(&current_hax_id1_path, &new_hacked_id1_path)?;
                }
            }

            let hacked_id1 = console.encoded_id1_readable();
            (
                title_dbs_exist,
                homemenu_extdata_exists,
                miimaker_extdata_exists,
            ) = sanity_check(sd_card, &id0, &hacked_id1)?;
            let sanity_ok = title_dbs_exist && homemenu_extdata_exists && miimaker_extdata_exists;

            let trigger_file_path =
                format!("Nintendo 3DS/{id0}/{hacked_id1}/extdata/{TRIGGER_FILE}");
            if sd_card.file_exists(&trigger_file_path)? {
                hax_state = HaxState::Injected;
            } else if sanity_ok {
                hax_state = HaxState::Ready;
            } else {
                hax_state = HaxState::NotReady;
            }
        }
    }

    let Some(id1) = id1 else {
        return Err(MSET9Error::UserError(
            "Couldn't find ID1 folder! Ensure that you are running this script from the root of the SD card.\nIf that doesn't work, eject the SD card, and put it back in your console. Turn it on and off again, then rerun this script.".to_string(),
            3,
        ));
    };
    if id1_count != 1 {
        return Err(MSET9Error::UserError(
            format!(
                "You don't have 1 ID1 in your Nintendo 3DS folder, you have {id0_count}!\nConsult: https://3ds.hacks.guide/troubleshooting#installing-boot9strap-mset9 for help!"
            ),
            5,
        ));
    }

    mainmenu(
        sd_card,
        &id0,
        &id1,
        &console,
        hax_state,
        title_dbs_exist,
        homemenu_extdata_exists,
        miimaker_extdata_exists,
    )?;

    Ok(())
}

fn mainmenu(
    sd_card: &SdCard,
    id0: &str,
    id1: &str,
    console: &Console,
    mut hax_state: HaxState,
    title_dbs_exist: bool,
    homemenu_extdata_exists: bool,
    miimaker_extdata_exists: bool,
) -> Result<(), MSET9Error> {
    header()?;
    let term = Term::stdout();
    term.write_line(&format!("Using: {}", style(&console).green()))?;
    term.write_line("")?;
    term.write_line(&format!("Current state: {}", style(&hax_state).yellow()))?;

    action_promt(&hax_state)?;

    let action = Input::<u32>::new()
        .with_prompt("Action")
        .validate_with(|input: &u32| -> Result<(), &str> {
            if !(0..=5).contains(input) {
                return Err("Please enter a number between 0 and 5");
            }
            if *input == 1 && hax_state != HaxState::NotCreated {
                return Err("Hacked ID1 already exists.");
            }
            if *input == 2 && hax_state == HaxState::NotCreated {
                return Err("Can't do that now!");
            }
            if *input == 3 && hax_state != HaxState::Ready {
                return Err("Can't do that now!");
            }
            if *input == 4 && hax_state < HaxState::Ready {
                return Err("Can't do that now!");
            }
            if *input == 5 && (hax_state == HaxState::NotCreated || hax_state == HaxState::Injected)
            {
                return Err("Can't do that now!");
            }
            Ok(())
        })
        .interact_text()?;

    match action {
        1 => create_hax_id1(sd_card, id0, id1, &console.encoded_id1_readable())?,
        2 => report_sanity(
            title_dbs_exist,
            homemenu_extdata_exists,
            miimaker_extdata_exists,
        )?,
        3 => inject_trigger(
            sd_card,
            &format!("Nintendo 3DS/{id0}/{}", console.encoded_id1_readable()),
        )?,
        4 => {
            let removed = remove_trigger(
                sd_card,
                &format!("Nintendo 3DS/{id0}/{}", console.encoded_id1_readable()),
            )?;
            if removed {
                hax_state = HaxState::Removed;
                mainmenu(
                    sd_card,
                    id0,
                    id1,
                    console,
                    hax_state,
                    title_dbs_exist,
                    homemenu_extdata_exists,
                    miimaker_extdata_exists,
                )?;
            }
        }
        5 => remove_mset9(sd_card, id0, id1, &console.encoded_id1_readable())?,
        0 => {}
        _ => unreachable!(),
    };

    Ok(())
}

fn create_hax_id1(
    sd_card: &SdCard,
    id0: &str,
    id1: &str,
    hacked_id1: &str,
) -> Result<(), MSET9Error> {
    let term = Term::stdout();
    term.write_line(&style("=== DISCLAIMER ===").red().bold().to_string())?;
    term.write_line("This process will temporarily reset all your 3DS data.")?;
    term.write_line("All your applications and themes will disappear.")?;
    term.write_line("This is perfectly normal, and if everything goes right, it will re-appear")?;
    term.write_line("at the end of the process.")?;
    term.write_line("")?;
    term.write_line("In any case, it is highly recommended to make a backup of your SD card's contents to a folder on your PC.")?;
    term.write_line("(Especially the 'Nintendo 3DS' folder.)")?;
    term.write_line("")?;

    term.write_line(&format!("Input {} to continue", style("1").green()))?;
    term.write_line(&format!("Input {} to exit", style("0").red()))?;

    let choice: u32 = Input::<u32>::new()
        .with_prompt("Your choice")
        .validate_with(|input: &u32| -> Result<(), &str> {
            if *input != 0 && *input != 1 {
                return Err("Please enter either 0 or 1");
            }
            Ok(())
        })
        .interact_text()?;

    if choice == 0 {
        term.write_line("Cancelled!")?;
        return Ok(());
    }

    // TODO: Handle create errors here more gracefully.

    info("Creating hacked ID1...")?;

    sd_card.create_dir(&format!("Nintendo 3DS/{id0}/{hacked_id1}"))?;
    sd_card.create_dir(&format!("Nintendo 3DS/{id0}/{hacked_id1}/dbs"))?;

    info("Creating dummy databases...")?;

    sd_card.create_file(
        &format!("Nintendo 3DS/{id0}/{hacked_id1}/dbs/title.db"),
        None,
    )?;
    sd_card.create_file(
        &format!("Nintendo 3DS/{id0}/{hacked_id1}/dbs/import.db"),
        None,
    )?;

    if !id1.ends_with(ID1_BACKUP_SUFFIX) {
        sd_card.rename(
            &format!("Nintendo 3DS/{id0}/{id1}"),
            &format!("Nintendo 3DS/{id0}/{id1}{ID1_BACKUP_SUFFIX}"),
        )?;
    }

    info("Created hacked ID1")?;

    Ok(())
}

const HOMEMENU_EXTDATA: [usize; 6] = [0x8F, 0x98, 0x82, 0xA1, 0xA9, 0xB1];
const MIIMAKER_EXTADATA: [usize; 6] = [0x217, 0x227, 0x207, 0x267, 0x277, 0x287];

fn sanity_check(
    sd_card: &SdCard,
    id0: &str,
    hacked_id1: &str,
) -> Result<(bool, bool, bool), MSET9Error> {
    let hacked_id1_path = format!("Nintendo 3DS/{id0}/{hacked_id1}");
    let title_db_exists = check_file(
        sd_card,
        &format!("{hacked_id1_path}/dbs/title.db"),
        Some(0x31E400),
    )?;
    let import_db_exists = check_file(
        sd_card,
        &format!("{hacked_id1_path}/dbs/import.db"),
        Some(0x31E400),
    )?;
    let title_dbs_exist = title_db_exists && import_db_exists;
    if !title_dbs_exist {
        sd_card.create_dir(&format!("Nintendo 3DS/{id0}/{hacked_id1}/dbs"))?;
        sd_card.create_file(
            &format!("Nintendo 3DS/{id0}/{hacked_id1}/dbs/title.db"),
            None,
        )?;
        sd_card.create_file(
            &format!("Nintendo 3DS/{id0}/{hacked_id1}/dbs/import.db"),
            None,
        )?;
    }

    let mut homemenu_extdata_exists = false;
    for &extdata in HOMEMENU_EXTDATA.iter().chain(MIIMAKER_EXTADATA.iter()) {
        let homemenu_extdata_path = format!("{hacked_id1_path}/extdata/00000000/{extdata:08X}");
        if sd_card.dir_exists(&homemenu_extdata_path)? {
            homemenu_extdata_exists = true;
            break;
        }
    }

    let mut miimaker_extdata_exists = false;
    for &extdata in MIIMAKER_EXTADATA.iter() {
        let miimaker_extdata_path = format!("{hacked_id1_path}/extdata/00000000/{extdata:08X}");
        if sd_card.dir_exists(&miimaker_extdata_path)? {
            miimaker_extdata_exists = true;
            break;
        }
    }

    Ok((
        title_dbs_exist,
        homemenu_extdata_exists,
        miimaker_extdata_exists,
    ))
}

fn is_3ds_id(name: &str) -> bool {
    if name.graphemes(true).count() != 32 {
        return false;
    };

    name.chars()
        .all(|c| c.is_ascii_hexdigit() && (c.is_ascii_lowercase() || c.is_ascii_digit()))
}

fn check_file(sd_card: &SdCard, path: &str, filesize: Option<usize>) -> Result<bool, MSET9Error> {
    if !sd_card.file_exists(path)? {
        return Ok(false);
    }

    if let Some(expected_filesize) = filesize {
        if let Ok(filesize) = sd_card.get_file_size(path) {
            if filesize != expected_filesize as u64 {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }

    Ok(true)
}

fn inject_trigger(sd_card: &SdCard, hacked_id1_path: &str) -> Result<(), MSET9Error> {
    let trigger_file_path = format!("{hacked_id1_path}/extdata/{TRIGGER_FILE}");

    if sd_card.file_exists(&trigger_file_path)? {
        error("Trigger file already injected!")?;
        return Ok(());
    }

    info("Injecting trigger file...")?;

    sd_card.create_file(&trigger_file_path, Some("pls be haxxed mister arm9, thx"))?;

    info("MSET9 successfully injected!")?;

    Ok(())
}

fn remove_trigger(sd_card: &SdCard, hacked_id1_path: &str) -> Result<bool, MSET9Error> {
    let trigger_file_path = format!("{hacked_id1_path}/extdata/{TRIGGER_FILE}");

    if !sd_card.file_exists(&trigger_file_path)? {
        error("Trigger file already removed!")?;
        return Ok(false);
    }

    info("Removing trigger file...")?;

    sd_card.remove(&trigger_file_path)?;

    info("Removed trigger file")?;

    Ok(true)
}

fn remove_mset9(
    sd_card: &SdCard,
    id0: &str,
    id1: &str,
    hacked_id1: &str,
) -> Result<(), MSET9Error> {
    info("Removing MSET9...")?;

    let id1_path = format!("Nintendo 3DS/{id0}/{id1}");
    let hacked_id1_path = format!("Nintendo 3DS/{id0}/{hacked_id1}");
    if sd_card.dir_exists(&hacked_id1_path)? {
        if !sd_card.dir_exists(&format!("{id1_path}/dbs"))? {
            info("Moving databases to user ID1...")?;
            sd_card.rename(
                &format!("{hacked_id1_path}/dbs"),
                &format!("{id1_path}/dbs"),
            )?;
        }
        info("Deleting hacked ID1...")?;
        sd_card.remove_tree(&hacked_id1_path)?;
    }
    if sd_card.dir_exists(&id1_path)? && id1.ends_with(ID1_BACKUP_SUFFIX) {
        info("Renaming original ID1...")?;
        sd_card.rename(
            &id1_path,
            &format!("Nintendo 3DS/{id0}/{}", id1.get(..32).unwrap()),
        )?;
    }

    info("MSET9 removed successfully!")?;

    Ok(())
}
