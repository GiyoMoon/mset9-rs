use console::{Term, style};

pub enum MSET9Error {
    InternalError(String),
    UserError(String, u32),
}

impl MSET9Error {
    pub fn report(&self) {
        let term = Term::stdout();
        match self {
            MSET9Error::InternalError(error) => {
                term.write_line(
                    &style("====== Unexpected Error ======")
                        .red()
                        .bold()
                        .to_string(),
                )
                .unwrap();
                term.write_line(&format!(
                    "{} {}",
                    style("==>").red(),
                    &error.replace('\n', &format!("\n{} ", style("==>").red()))
                ))
                .unwrap();
                term.write_line(&style("======================").red().to_string())
                    .unwrap();
            }
            MSET9Error::UserError(error, id) => {
                term.write_line(
                    &style(format!("====== Error {id:0>2} ======"))
                        .yellow()
                        .bold()
                        .to_string(),
                )
                .unwrap();
                term.write_line(&format!(
                    "{} {}",
                    style("==>").yellow(),
                    &error.replace('\n', &format!("\n{} ", style("==>").yellow()))
                ))
                .unwrap();
                term.write_line(&style("======================").yellow().to_string())
                    .unwrap();
            }
        }
        term.write_line("Press any key to exit").unwrap();
        term.read_key().unwrap();
    }
}

impl From<std::io::Error> for MSET9Error {
    fn from(value: std::io::Error) -> Self {
        Self::InternalError(format!("I/O Error: {value}"))
    }
}

impl From<dialoguer::Error> for MSET9Error {
    fn from(value: dialoguer::Error) -> Self {
        Self::InternalError(format!("Dialoguer Error: {value}"))
    }
}
