//! Picking a clip, naming the pet it belongs to, and importing it in the background;
//! and sharing whole pets as `.pawpal` files.

use std::sync::mpsc;

use super::App;
use crate::data::{clip_importer, pet_share, skin};
use crate::platform;

pub enum ImportMsg {
    Progress(String),
    Done(String, Result<String, String>),
}

impl App {
    pub fn import_clip_ui(&mut self, preset: Option<String>, action: &'static str) {
        let Some(file) = rfd::FileDialog::new()
            .set_title("Choose a short clip of your cat (2–6 seconds, plain background works best)")
            .add_filter("Clips", &["mp4", "mov", "m4v", "webm", "mkv", "avi", "gif", "webp", "png", "apng"])
            .pick_file()
        else {
            return;
        };
        let name = match preset {
            Some(name) => name,
            None => {
                let default = self.skins.keys().next().cloned().unwrap_or_else(|| "My Cat".into());
                let message = "Which pet is this? Use the same name for every clip of the same cat. PawPal removes the background automatically.";
                let Some(name) = platform::prompt_text("Import a Clip", message, &default) else { return };
                name
            }
        };
        let skin_name: String = name.chars().filter(|c| !matches!(c, '/' | ':' | '\\')).collect::<String>().trim().to_string();
        if skin_name.is_empty() { return }

        self.set_status("🐾 …");
        let (tx, rx) = mpsc::channel();
        self.import_rx = Some(rx);
        std::thread::spawn(move || {
            let progress = tx.clone();
            let result = clip_importer::import_clip(&file, &skin_name, action, &move |text| {
                let _ = progress.send(ImportMsg::Progress(text));
            });
            let _ = tx.send(ImportMsg::Done(skin_name, result));
        });
    }

    /// Shows import progress in the tray and picks up the finished clip.
    pub fn poll_import(&mut self) {
        let Some(rx) = &self.import_rx else { return };
        let msgs: Vec<ImportMsg> = rx.try_iter().collect();
        for msg in msgs {
            match msg {
                ImportMsg::Progress(text) => {
                    let short = text.split(' ').next_back().unwrap_or("").to_string();
                    self.set_status(&format!("🐾 {short}"));
                }
                ImportMsg::Done(skin_name, Ok(message)) => {
                    self.import_rx = None;
                    self.finish_import(&skin_name, message);
                }
                ImportMsg::Done(_, Err(error)) => {
                    self.import_rx = None;
                    self.set_status("🐾");
                    platform::alert("Couldn't import this clip", &error, &["OK"], true);
                }
            }
        }
    }

    fn finish_import(&mut self, skin_name: &str, message: String) {
        self.set_status("🐾");
        self.reload_skins();
        if !self.world.pets.iter().any(|p| p.skin.as_ref().is_some_and(|s| s.name == skin_name)) {
            self.adopt(skin_name);
        }
        if let Some(p) = self.world.pets.iter_mut().find(|p| p.skin.as_ref().is_some_and(|s| s.name == skin_name)) {
            p.say(message, 4.0);
        }
    }

    /// "📤 Share…": packs a pet into a `.pawpal` file to send to a friend.
    pub fn share_pet(&mut self, name: &str) {
        let Some(mut out) = rfd::FileDialog::new()
            .set_title(format!("Share {name}"))
            .set_file_name(format!("{name}.{}", pet_share::EXTENSION))
            .add_filter("PawPal pet", &[pet_share::EXTENSION])
            .save_file()
        else {
            return;
        };
        if out.extension().is_none_or(|e| e != pet_share::EXTENSION) { out.set_extension(pet_share::EXTENSION); }
        match pet_share::export(&skin::root().join(name), name, &out) {
            Ok(()) => self.world.tell(&format!("Packed up {name}! Send the .pawpal file to a friend 🐾")),
            Err(e) => { platform::alert("Couldn't share this pet", &e, &["OK"], true); }
        }
    }

    /// "Import a Shared Pet…": unpacks a friend's `.pawpal` file and adopts the pet.
    pub fn import_shared_ui(&mut self) {
        let Some(file) = rfd::FileDialog::new()
            .set_title("Choose a shared pet")
            .add_filter("PawPal pet", &[pet_share::EXTENSION])
            .pick_file()
        else {
            return;
        };
        match pet_share::import(&file, &skin::root()) {
            Ok(name) => {
                self.reload_skins();
                self.adopt(&name);
            }
            Err(e) => { platform::alert("Couldn't import this pet", &e, &["OK"], true); }
        }
    }

    /// Reads the clip library again and hands every pet its fresh skin.
    pub fn reload_skins(&mut self) {
        self.skins = skin::load_all();
        self.stage_mut(|s| s.reload_clips());
        for p in &mut self.world.pets {
            if let Some(name) = p.skin.as_ref().map(|s| s.name.clone()) {
                p.skin = self.skins.get(&name).cloned();
            }
        }
    }
}
