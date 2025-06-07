use console::{Term, style};
use dialoguer::Input;

use crate::{console::Console, error::MSET9Error, mset9::HaxState};

const VERSION: &str = "2.0.0";

pub fn info(message: &str) -> Result<(), MSET9Error> {
    let term = Term::stdout();
    term.write_line(&format!("==> {}", message.replace('\n', "\n==> ")))?;
    Ok(())
}

pub fn error(message: &str) -> Result<(), MSET9Error> {
    let term = Term::stdout();
    term.write_line(&format!("==> {}", style(message).red().bold()))?;
    Ok(())
}

pub fn header() -> Result<(), MSET9Error> {
    let term = Term::stdout();
    term.clear_screen()?;

    term.write_line(" ███╗   ███╗███████╗███████╗████████╗ █████╗  ")?;
    term.write_line(" ████╗ ████║██╔════╝██╔════╝╚══██╔══╝██╔══██╗ ")?;
    term.write_line(" ██╔████╔██║███████╗█████╗     ██║   ╚██████║ ")?;
    term.write_line(" ██║╚██╔╝██║╚════██║██╔══╝     ██║    ╚═══██║ ")?;
    term.write_line(" ██║ ╚═╝ ██║███████║███████╗   ██║    █████╔╝ ")?;
    term.write_line(" ╚═╝     ╚═╝╚══════╝╚══════╝   ╚═╝    ╚════╝  ")?;
    term.write_line("██████████████████████████████████████████████")?;
    term.write_line(&format!(
        "MSET9 v{VERSION} by zoogie, Aven, DannyAAM and thepikachugamer"
    ))?;
    term.write_line("ported to Rust by Jasmin (GiyoMoon)")?;
    term.write_line("")?;
    Ok(())
}

pub fn ask_for_console() -> Result<Console, MSET9Error> {
    console_promt()?;

    let console = Input::<u32>::new()
        .with_prompt("Your console model")
        .validate_with(|input: &u32| -> Result<(), &str> {
            if !(1..=4).contains(input) {
                return Err("Please enter a number between 1 and 4");
            }
            Ok(())
        })
        .interact_text()?;

    Ok(match console {
        1 => Console::Old3DSLatest,
        2 => Console::New3DSLatest,
        3 => Console::Old3DSOld,
        4 => Console::New3DSOld,
        _ => unreachable!(),
    })
}

fn console_promt() -> Result<(), MSET9Error> {
    let term = Term::stdout();
    term.write_line("What is your console model and version?")?;
    term.write_line(&format!(
        "{} has two shoulder buttons (L and R)",
        style("Old 3DS/2DS").cyan()
    ))?;
    term.write_line(&format!(
        "{} has four shoulder buttons (L, R, ZL, ZR)",
        style("New 3DS/2DS").blue()
    ))?;
    term.write_line("")?;
    term.write_line(&format!(
        "=== Please type in a {} then hit return ===",
        style("number").green()
    ))?;
    term.write_line(&format!(
        "{}: {}, {}",
        style("1").green(),
        style(Console::Old3DSLatest.model()).cyan(),
        Console::Old3DSLatest.version()
    ))?;
    term.write_line(&format!(
        "{}: {}, {}",
        style("2").green(),
        style(Console::New3DSLatest.model()).cyan(),
        Console::New3DSLatest.version()
    ))?;
    term.write_line(&format!(
        "{}: {}, {}",
        style("3").green(),
        style(Console::Old3DSOld.model()).cyan(),
        Console::Old3DSOld.version()
    ))?;
    term.write_line(&format!(
        "{}: {}, {}",
        style("4").green(),
        style(Console::New3DSOld.model()).cyan(),
        Console::New3DSOld.version()
    ))?;
    Ok(())
}

pub fn action_promt(hax_state: &HaxState) -> Result<(), MSET9Error> {
    let term = Term::stdout();
    term.write_line("Please type in a number then hit return")?;
    term.write_line(&format!("{}: Create MSET9 ID1", style("1").green(),))?;
    term.write_line(&format!("{}: Check MSET9 status", style("2").green(),))?;
    term.write_line(&format!("{}: Inject trigger file", style("3").green(),))?;
    term.write_line(&format!("{}: Remove trigger file", style("4").green(),))?;
    if hax_state != &HaxState::Injected {
        term.write_line(&format!("{}: Remove MSET9", style("5").green(),))?;
    }
    term.write_line(&format!("{}: Exit", style("0").green(),))?;
    Ok(())
}

pub fn report_sanity(
    title_dbs_exist: bool,
    homemenu_extdata_exists: bool,
    miimaker_extdata_exists: bool,
) -> Result<(), MSET9Error> {
    if title_dbs_exist {
        info("Title database: OK")?;
    } else {
        error("Title database: Not initialized!")?;
        info("Please power on your console with your SD inserted, open System Settings,")?;
        info("navigate to Data Management -> Nintendo 3DS -> Software, then select Reset.")?;
    }

    if homemenu_extdata_exists {
        info("Home Menu extdata: OK")?;
    } else {
        error("Home Menu extdata: Missing!")?;
        info("Please power on your console with your SD inserted, then check again.")?;
        info("If this does not work, your SD card may need to be reformatted.")?;
    }

    if miimaker_extdata_exists {
        info("Mii Maker extdata: OK")?;
    } else {
        error("Mii Maker extdata: Missing!")?;
        info("Please power on your console with your SD inserted, then launch Mii Maker.")?;
    }

    Ok(())
}
