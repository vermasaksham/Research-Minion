use crate::{
    app::{INK, MUTED, PAPER},
    editor::Editor,
    markdown::{self, Format},
    note_images,
};
use eframe::egui::{
    self, RichText,
    text::{CCursor, CCursorRange},
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::{collections::HashMap, path::Path};

pub struct NotesView {
    preview: bool,
    cache: CommonMarkCache,
    text_id: egui::Id,
    image_textures: HashMap<String, egui::TextureHandle>,
    image_error: Option<String>,
}

impl Default for NotesView {
    fn default() -> Self {
        Self {
            preview: false,
            cache: CommonMarkCache::default(),
            text_id: egui::Id::new(uuid::Uuid::new_v4()),
            image_textures: HashMap::new(),
            image_error: None,
        }
    }
}

impl NotesView {
    pub fn show(&mut self, ui: &mut egui::Ui, editor: &mut Editor, height: f32, workspace: &Path) {
        let top = ui.cursor().top();
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.preview, false, "Write");
            ui.selectable_value(&mut self.preview, true, "Preview");
            ui.menu_button("Markdown help", |ui| {
                ui.label("# Heading\n**bold** · *italic* · ~~strikethrough~~\n- Bullet · 1. Numbered item\n> Quoted text\n`inline code` · ``` fenced code blocks\n[label](https://example.com)\n- [ ] Task · - [x] Completed task");
                ui.separator();
                ui.label("Tables: separate columns with | and add a |---|---| separator row.");
                ui.label("Select digits or + - = ( ) for subscript/superscript. Ctrl+B / Ctrl+I / Ctrl+K also work while writing.");
                ui.label("Add picture copies a PNG/JPEG into this profile. Save the note to keep its reference.");
                ui.label("Preview reads the current draft. Save entry stores the Markdown source.");
            });
        });
        let mut format = None;
        if !self.preview {
            ui.horizontal_wrapped(|ui| {
                for (label, action, hint) in [
                    ("Bold", Format::Bold, "Bold · Ctrl+B"),
                    ("Italic", Format::Italic, "Italic · Ctrl+I"),
                    ("Heading", Format::Heading, "Level-two heading"),
                    ("Bullets", Format::Bullets, "Bullet list"),
                    ("1. List", Format::Numbered, "Numbered list"),
                    ("Quote", Format::Quote, "Block quote"),
                    (
                        "Code",
                        Format::Code,
                        "Inline code or fenced code for multiple lines",
                    ),
                    ("Link", Format::Link, "Link · Ctrl+K"),
                    (
                        "Subscript",
                        Format::Subscript,
                        "Turn selected digits and signs into subscripts",
                    ),
                    (
                        "Superscript",
                        Format::Superscript,
                        "Turn selected digits and signs into superscripts",
                    ),
                ] {
                    if ui.small_button(label).on_hover_text(hint).clicked() {
                        format = Some(action);
                    }
                }
                #[cfg(windows)]
                if ui.small_button("Add picture…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .set_title("Add a picture to this note")
                        .add_filter("Pictures", &["png", "jpg", "jpeg"])
                        .pick_file()
                {
                    match note_images::add(workspace, &path) {
                        Ok((reference, _)) => {
                            editor.body.push_str(&format!("\n\n{reference}\n"));
                            self.image_error = None;
                        }
                        Err(error) => self.image_error = Some(error),
                    }
                }
            });
            if ui.memory(|m| m.has_focus(self.text_id)) {
                for (key, action) in [
                    (egui::Key::B, Format::Bold),
                    (egui::Key::I, Format::Italic),
                    (egui::Key::K, Format::Link),
                ] {
                    if ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key)) {
                        format = Some(action);
                    }
                }
            }
        }
        if let Some(error) = &self.image_error {
            ui.colored_label(egui::Color32::DARK_RED, error);
        }
        if let Some(action) = format {
            let mut state = egui::TextEdit::load_state(ui.ctx(), self.text_id).unwrap_or_default();
            let range = state
                .cursor
                .char_range()
                .unwrap_or_else(|| CCursorRange::one(CCursor::new(editor.body.chars().count())));
            let [start, end] = range.sorted_cursors();
            let mut undoer = state.undoer();
            undoer.add_undo(&(range, editor.body.clone()));
            let selection = markdown::format(&mut editor.body, start.index..end.index, action);
            let range =
                CCursorRange::two(CCursor::new(selection.start), CCursor::new(selection.end));
            undoer.add_undo(&(range, editor.body.clone()));
            state.set_undoer(undoer);
            state.cursor.set_char_range(Some(range));
            state.store(ui.ctx(), self.text_id);
        }
        ui.add_space(5.0);
        let height = (height - (ui.cursor().top() - top)).max(140.0);
        egui::ScrollArea::both()
            .id_salt(("markdown-body", self.text_id, self.preview))
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.set_min_height(height);
                if self.preview {
                    egui::Frame::new()
                        .fill(PAPER)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            if editor.body.is_empty() {
                                ui.label(
                                    RichText::new("Write some Markdown to see its preview here.")
                                        .color(MUTED),
                                );
                            } else {
                                ui.style_mut().url_in_tooltip = true;
                                let (markdown, pictures) =
                                    note_images::split_markdown(&editor.body);
                                CommonMarkViewer::new().show(ui, &mut self.cache, &markdown);
                                for (caption, id) in pictures {
                                    if !self.image_textures.contains_key(&id)
                                        && let Some(path) =
                                            note_images::picture_path(workspace, &id)
                                        && let Ok(bytes) = std::fs::read(path)
                                        && let Ok(image) = image::load_from_memory(&bytes)
                                    {
                                        let rgba = image.to_rgba8();
                                        let texture = ui.ctx().load_texture(
                                            format!("note-picture-{}-{id}", workspace.display()),
                                            egui::ColorImage::from_rgba_unmultiplied(
                                                [rgba.width() as usize, rgba.height() as usize],
                                                rgba.as_raw(),
                                            ),
                                            egui::TextureOptions::LINEAR,
                                        );
                                        self.image_textures.insert(id.clone(), texture);
                                    }
                                    ui.add_space(8.0);
                                    if let Some(texture) = self.image_textures.get(&id) {
                                        ui.add(
                                            egui::Image::new(texture)
                                                .max_width(ui.available_width().min(720.0)),
                                        );
                                        ui.label(caption);
                                    } else {
                                        ui.colored_label(
                                            egui::Color32::DARK_RED,
                                            "Picture missing from this profile",
                                        );
                                    }
                                }
                            }
                        });
                } else {
                    let response = ui.add_sized(
                        [ui.available_width(), height],
                        egui::TextEdit::multiline(&mut editor.body)
                            .id(self.text_id)
                            .font(egui::TextStyle::Monospace)
                            .text_color(INK)
                            .hint_text("Write your research notes in Markdown…")
                            .desired_width(f32::INFINITY),
                    );
                    // Restore focus after TextEdit handles the toolbar's outside click.
                    if format.is_some() {
                        response.request_focus();
                    }
                }
            });
    }
}
