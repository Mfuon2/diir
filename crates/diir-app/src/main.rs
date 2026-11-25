#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod components;
mod platform;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("Diir could not start: {error}");
        std::process::exit(1);
    }
}
