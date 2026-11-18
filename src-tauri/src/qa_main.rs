#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app_updates;
mod desktop;
#[cfg(feature = "updater-qa")]
mod updater_qa;
fn main() {
    desktop::run_app();
}
