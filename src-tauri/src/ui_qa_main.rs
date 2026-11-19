#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app_updates;
mod desktop;
mod ui_qa;
fn main() {
    desktop::run_app();
}
