mod app;
mod cli;
mod config;
mod domain;
mod use_cases;
mod change_detection;
mod platform;
mod parsers;
mod output;
mod support;

use std::process;

fn main() {
    let exit_code = app::run();
    process::exit(exit_code);
}
