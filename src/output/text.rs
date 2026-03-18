use crate::output::json::Envelope;
use serde::Serialize;

pub struct TextPresenter {
    pub no_color: bool,
}

impl TextPresenter {
    pub fn print_ok(&self, msg: &str) {
        if self.no_color {
            println!("OK: {msg}");
        } else {
            println!("\x1b[32mOK\x1b[0m: {msg}");
        }
    }

    pub fn print_error(&self, msg: &str) {
        if self.no_color {
            eprintln!("ERROR: {msg}");
        } else {
            eprintln!("\x1b[31mERROR\x1b[0m: {msg}");
        }
    }

    pub fn print_info(&self, msg: &str) {
        println!("{msg}");
    }
}

pub struct JsonPresenter;

impl JsonPresenter {
    pub fn print<T: Serialize>(&self, envelope: &Envelope<T>) {
        match serde_json::to_string_pretty(envelope) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("JSON serialization error: {e}"),
        }
    }
}
