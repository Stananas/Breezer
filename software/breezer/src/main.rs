// On Windows, don't flash a console window alongside the GUI app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), breezer::error::Error> {
    breezer::app::run()
}