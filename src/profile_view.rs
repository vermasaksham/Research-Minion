use crate::{
    app::{MUTED, TEAL, primary_button},
    profiles::{self, Profile},
};
use eframe::egui::{self, Color32, RichText, TextureHandle, Vec2};

pub fn avatar_texture(ctx: &egui::Context, key: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let png = profiles::prepare_picture(bytes).ok()?;
    let image = image::load_from_memory(&png).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Some(ctx.load_texture(
        key,
        egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::LINEAR,
    ))
}

pub fn avatar(ui: &mut egui::Ui, name: &str, texture: Option<&TextureHandle>, size: f32) {
    if let Some(texture) = texture {
        ui.add(
            egui::Image::new(texture)
                .fit_to_exact_size(Vec2::splat(size))
                .corner_radius(size / 2.0),
        );
    } else {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), size / 2.0, TEAL);
        let initials: String = name
            .split_whitespace()
            .take(2)
            .filter_map(|part| part.chars().next())
            .flat_map(char::to_uppercase)
            .collect();
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if initials.is_empty() { "?" } else { &initials },
            egui::FontId::proportional(size * 0.36),
            Color32::WHITE,
        );
    }
}

pub struct ProfileForm {
    pub id: Option<String>,
    pub full_name: String,
    pub picture_png: Option<Vec<u8>>,
    picture_path: String,
    loaded_path: String,
    baseline_name: String,
    baseline_picture: Option<Vec<u8>>,
    preview: Option<TextureHandle>,
    picture_error: Option<String>,
}
impl Default for ProfileForm {
    fn default() -> Self {
        Self::new(None)
    }
}
impl ProfileForm {
    pub fn new(profile: Option<&Profile>) -> Self {
        let name = profile.map(|p| p.full_name.clone()).unwrap_or_default();
        let picture = profile.and_then(|p| p.picture_png.clone());
        Self {
            id: profile.map(|p| p.id.clone()),
            full_name: name.clone(),
            picture_png: picture.clone(),
            picture_path: String::new(),
            loaded_path: String::new(),
            baseline_name: name,
            baseline_picture: picture,
            preview: None,
            picture_error: None,
        }
    }
    pub fn is_dirty(&self) -> bool {
        self.full_name != self.baseline_name
            || self.picture_png != self.baseline_picture
            || self.picture_path != self.loaded_path
    }
    pub fn resolve_picture(&mut self) -> Result<(), String> {
        if !self.picture_path.trim().is_empty() && self.picture_path != self.loaded_path {
            let picture =
                profiles::picture_from_file(std::path::Path::new(self.picture_path.trim()))
                    .map_err(|error| error.to_string())?;
            self.picture_png = Some(picture);
            self.loaded_path = self.picture_path.clone();
            self.preview = None;
            self.picture_error = None;
        }
        Ok(())
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        error: Option<&str>,
        allow_cancel: bool,
    ) -> Option<bool> {
        let mut result = None;
        ui.strong("Full name");
        ui.add(
            egui::TextEdit::singleline(&mut self.full_name)
                .hint_text("Enter your full name")
                .margin(egui::vec2(10.0, 10.0))
                .desired_width(f32::INFINITY),
        );
        ui.add_space(12.0);
        ui.strong("User picture");
        if self.preview.is_none()
            && let Some(bytes) = &self.picture_png
        {
            self.preview = avatar_texture(ui.ctx(), "profile-picture-preview", bytes);
        }
        ui.horizontal(|ui| {
            avatar(ui, &self.full_name, self.preview.as_ref(), 72.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("PNG or JPEG · optional").color(MUTED));
                #[cfg(windows)]
                if ui.button("Choose picture…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .set_title("Choose your profile picture")
                        .add_filter("Pictures", &["png", "jpg", "jpeg"])
                        .pick_file()
                {
                    self.picture_path = path.to_string_lossy().into_owned();
                    if let Err(error) = self.resolve_picture() {
                        self.picture_error = Some(error);
                    }
                }
                if self.picture_png.is_some() && ui.button("Remove picture").clicked() {
                    self.picture_png = None;
                    self.picture_path.clear();
                    self.loaded_path.clear();
                    self.preview = None;
                    self.picture_error = None;
                }
                if self.picture_png.is_none() {
                    ui.label(
                        RichText::new("Your initials are shown until you choose a picture.")
                            .size(12.0)
                            .color(MUTED),
                    );
                }
            });
        });
        ui.collapsing("Choose picture using a file path", |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.picture_path)
                    .hint_text("Path to a PNG or JPEG picture")
                    .desired_width(f32::INFINITY),
            );
            if ui.button("Load picture").clicked()
                && let Err(error) = self.resolve_picture()
            {
                self.picture_error = Some(error);
            }
        });
        if let Some(path) = ui.ctx().input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .find_map(|file| file.path.clone())
        }) {
            self.picture_path = path.to_string_lossy().into_owned();
            if let Err(error) = self.resolve_picture() {
                self.picture_error = Some(error);
            }
        }
        if let Some(error) = self.picture_error.as_deref().or(error) {
            ui.colored_label(Color32::DARK_RED, error);
        }
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if allow_cancel && ui.button("Cancel").clicked() {
                result = Some(false);
            }
            ui.add_enabled_ui(!self.full_name.trim().is_empty(), |ui| {
                if primary_button(
                    ui,
                    if self.id.is_some() {
                        "Save profile"
                    } else {
                        "Create profile"
                    },
                )
                .clicked()
                {
                    result = Some(true);
                }
            });
        });
        ui.label(
            RichText::new("This profile and its research stay on this computer.")
                .color(MUTED)
                .size(12.0),
        );
        result
    }
}
