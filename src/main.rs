mod app;
mod change_detection;
mod cli;
mod config;
mod domain;
mod output;
mod parsers;
mod platform;
mod support;
mod use_cases;

use std::process;

fn main() {
    let exit_code = app::run();
    process::exit(exit_code);
}
