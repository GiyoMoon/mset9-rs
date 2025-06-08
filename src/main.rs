use ::console::Term;
use checks::run_checks;
use sdcard::SdCard;

mod checks;
mod console;
mod error;
mod mset9;
mod sdcard;
mod term;

fn main() {
    let script_path = std::env::current_exe().unwrap();
    let sd_root = script_path.parent().unwrap();

    let fs_check_result = run_checks(sd_root);
    if let Err(e) = fs_check_result {
        e.report();
        return;
    }

    let sd_card_result = SdCard::setup(sd_root.to_str().unwrap().to_string());
    let sd_card = match sd_card_result {
        Ok(sdcard) => sdcard,
        Err(e) => {
            e.report();
            return;
        }
    };

    let mset9_result = mset9::launch(&sd_card);
    if let Err(e) = mset9_result {
        e.report();
    }

    let cleanup_result = sd_card.cleanup();
    if let Err(e) = cleanup_result {
        e.report();
    }

    let term = Term::stdout();
    term.write_line("=== Press any key to exit").unwrap();
    term.read_key().unwrap();
}
