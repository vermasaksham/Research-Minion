use crate::{
    app::{INK, MUTED, PAPER},
    editor::Editor,
    markdown::{self, Format},
};
use eframe::egui::{
    self, RichText,
    text::{CCursor, CCursorRange},
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

pub struct NotesView {
    preview: bool,
    cache: CommonMarkCache,
    text_id: egui::Id,
}

impl Default for NotesView {
    fn default() -> Self {
        Self {
            preview: false,
            cache: CommonMarkCache::default(),
            text_id: egui::Id::new(uuid::Uuid::new_v4()),
        }
    }
}

impl NotesView {
    pub fn show(&mut self, ui: &mut egui::Ui, editor: &mut Editor, height: f32) {
        let top = ui.cursor().top();
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.preview, false, "Write");
            ui.selectable_value(&mut self.preview, true, "Preview");
            ui.menu_button("Markdown help", |ui| {
                ui.label("# Heading\n**bold** · *italic* · ~~strikethrough~~\n- Bullet · 1. Numbered item\n> Quoted text\n`inline code` · ``` fenced code blocks\n[label](https://example.com)\n- [ ] Task · - [x] Completed task");
                ui.separator();
                ui.label("Tables: separate columns with | and add a |---|---| separator row.");
                ui.label("Select text, then choose formatting. Ctrl+B / Ctrl+I / Ctrl+K also work while writing.");
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
                ] {
                    if ui.small_button(label).on_hover_text(hint).clicked() {
                        format = Some(action);
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
                                CommonMarkViewer::new().show(ui, &mut self.cache, &editor.body);
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
