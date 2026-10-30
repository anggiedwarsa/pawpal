// PawPal — a free, realistic desktop pet that lives on your Dock or taskbar.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod data;
mod domain;
mod engine;
mod platform;
mod render;
mod support;
#[cfg(test)]
mod testing;

use std::path::Path;

use data::{clip_importer, pet_share, sheet_importer, skin};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // `pawpal --import clip.mp4 "My Cat" walk` imports a clip from the command line.
    if let Some(i) = args.iter().position(|a| a == "--import").filter(|i| i + 3 < args.len()) {
        let result = clip_importer::import_clip(Path::new(&args[i + 1]), &args[i + 2], &args[i + 3], &|line| println!("{line}"));
        finish(result);
    }

    // `pawpal --import-sheet sheet.png "My Cat" "walk=9@10/30,sit=0@5" ["Art by …"]` imports a hand-drawn sprite sheet.
    if let Some(i) = args.iter().position(|a| a == "--import-sheet").filter(|i| i + 3 < args.len()) {
        let spec = sheet_importer::parse(&args[i + 3]);
        let result = sheet_importer::import_sheet(Path::new(&args[i + 1]), &args[i + 2], &spec, args.get(i + 4).map(String::as_str));
        finish(result);
    }

    // `pawpal --import-pet Mochi.pawpal` adds a pet a friend shared.
    if let Some(i) = args.iter().position(|a| a == "--import-pet").filter(|i| i + 1 < args.len()) {
        let result = pet_share::import(Path::new(&args[i + 1]), &skin::root());
        finish(result.map(|name| format!("Imported {name}. Adopt it from ➕ Adopt / Import a Pet in the 🐾 menu.")));
    }

    app::run();
}

fn finish(result: Result<String, String>) -> ! {
    match result {
        Ok(message) => {
            println!("{message}");
            std::process::exit(0)
        }
        Err(e) => {
            println!("Import failed: {e}");
            std::process::exit(1)
        }
    }
}
