use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};

use crate::{
    app::{INK, MUTED, PAPER, TEAL, primary_button},
    cvt::{CvtConditions, TemperatureError, TemperatureScale, temperature_text},
    materials_view::{self, ElementTarget},
    model::{CvtExperiment, ExperimentDetails, HeatEffect},
};

const SOURCE: Color32 = Color32::from_rgb(191, 103, 58);
const SINK: Color32 = Color32::from_rgb(40, 119, 149);

pub struct ExperimentForm {
    pub experiment_date: String,
    initial_date: String,
    pub name: String,
    pub source: String,
    pub sink: String,
    pub details: ExperimentDetails,
    target: ElementTarget,
    materials_page: bool,
    ui_id: uuid::Uuid,
}

impl Default for ExperimentForm {
    fn default() -> Self {
        let initial_date = chrono::Local::now().date_naive().to_string();
        Self {
            experiment_date: initial_date.clone(),
            initial_date,
            name: String::new(),
            source: String::new(),
            sink: String::new(),
            details: ExperimentDetails::default(),
            target: ElementTarget::default(),
            materials_page: false,
            ui_id: uuid::Uuid::new_v4(),
        }
    }
}

pub enum FormAction {
    Save,
    Cancel,
    Changed,
}

impl ExperimentForm {
    pub fn is_dirty(&self) -> bool {
        self.experiment_date != self.initial_date
            || !self.name.is_empty()
            || !self.source.is_empty()
            || !self.sink.is_empty()
            || self.details != ExperimentDetails::default()
    }

    pub fn conditions(&self) -> Result<CvtConditions, TemperatureError> {
        CvtConditions::parse(&self.source, &self.sink)
    }

    pub fn date(&self) -> Result<Option<chrono::NaiveDate>, String> {
        let value = self.experiment_date.trim();
        if value.is_empty() {
            return Ok(None);
        }
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .ok()
            .filter(|date| date.to_string() == value)
            .map(Some)
            .ok_or_else(|| {
                "Use a valid experiment date in YYYY-MM-DD format, or leave it blank if unknown."
                    .into()
            })
    }

    pub fn show(&mut self, ctx: &egui::Context, error: Option<&str>) -> Option<FormAction> {
        let mut action = None;
        let mut changed = false;
        egui::Modal::new(egui::Id::new(("new-cvt-experiment", self.ui_id)))
            .frame(egui::Frame::popup(&ctx.style()).fill(Color32::WHITE).inner_margin(20).corner_radius(10.0))
            .show(ctx, |ui| {
            ui.set_width(820.0);
            ui.spacing_mut().item_spacing = Vec2::new(12.0, 9.0);
            ui.heading(RichText::new("New CVT experiment").size(25.0).color(INK));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.materials_page, false, "Conditions");
                ui.selectable_value(&mut self.materials_page, true, "Materials & transport agent");
            });
            ui.separator();
            let conditions = self.conditions();
            let date = self.date();
            egui::ScrollArea::vertical()
                .id_salt(("experiment-form-body", self.ui_id, self.materials_page))
                .max_height((ctx.content_rect().height() - 270.0).clamp(230.0, 530.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.add_space(8.0);
                    if self.materials_page {
                        ui.label(RichText::new("Materials are optional. Enter a formula or name, or choose elements below.").color(MUTED));
                        ui.add_space(8.0);
                        ui.columns(2, |columns| {
                            material_card(&mut columns[0], "Precursor", &mut self.details.precursor, &mut changed);
                            material_card(&mut columns[1], "Obtained material", &mut self.details.product, &mut changed);
                        });
                        ui.add_space(12.0);
                        ui.collapsing(RichText::new("Choose from the periodic table").strong(), |ui| {
                            changed |= materials_view::periodic_table(ui, &mut self.details, &mut self.target);
                        });
                        ui.add_space(16.0);
                        ui.strong("Transport agent");
                        changed |= ui.add(egui::TextEdit::singleline(&mut self.details.transport_agent)
                            .margin(egui::vec2(10.0, 9.0)).hint_text("Optional · formula or name, e.g. I2")
                            .desired_width(f32::INFINITY)).changed();
                    } else {
                        ui.strong("Experiment name");
                        changed |= ui.add(egui::TextEdit::singleline(&mut self.name)
                            .margin(egui::vec2(10.0, 9.0)).hint_text("Optional · a label is generated if blank")
                            .desired_width(f32::INFINITY)).changed();
                        ui.add_space(8.0);
                        ui.strong("Experiment date");
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::TextEdit::singleline(&mut self.experiment_date)
                                .margin(egui::vec2(10.0, 9.0)).desired_width(140.0).hint_text("YYYY-MM-DD")).changed();
                            ui.label(RichText::new("Leave blank if unknown").color(MUTED));
                        });
                        if let Err(error) = &date { ui.colored_label(Color32::DARK_RED, error); }
                        ui.add_space(14.0);
                        ui.columns(2, |columns| {
                            columns[0].strong("Source temperature · °C");
                            changed |= columns[0].add(egui::TextEdit::singleline(&mut self.source)
                                .font(egui::FontId::proportional(20.0)).margin(egui::vec2(12.0, 11.0))
                                .hint_text("e.g. 650").desired_width(f32::INFINITY)).changed();
                            columns[1].strong("Sink temperature · °C");
                            changed |= columns[1].add(egui::TextEdit::singleline(&mut self.sink)
                                .font(egui::FontId::proportional(20.0)).margin(egui::vec2(12.0, 11.0))
                                .hint_text("e.g. 600").desired_width(f32::INFINITY)).changed();
                        });
                        ui.add_space(8.0);
                        egui::Frame::new().fill(Color32::from_rgb(229, 241, 237)).corner_radius(6.0)
                            .inner_margin(12).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.strong("Temperature difference");
                                    ui.label(RichText::new(match &conditions {
                                        Ok(pair) => format!("ΔT = {} °C", temperature_text(pair.delta_c())),
                                        Err(_) => "ΔT = Source − Sink".into(),
                                    }).size(20.0).color(TEAL));
                                });
                            });
                        if let Err(error) = &conditions
                            && !self.source.is_empty() && !self.sink.is_empty() {
                            ui.colored_label(Color32::DARK_RED, error.to_string());
                        }
                        ui.add_space(12.0);
                        ui.strong("Thermal character").on_hover_text("Assigned by you, independently of the temperature difference.");
                        ui.horizontal_wrapped(|ui| {
                            for effect in [HeatEffect::Unspecified, HeatEffect::Endothermic, HeatEffect::Exothermic] {
                                changed |= ui.selectable_value(&mut self.details.heat_effect, effect, effect.label()).changed();
                            }
                        });
                    }
                    ui.add_space(6.0);
                });
            ui.separator();
            if let Some(error) = error { ui.colored_label(Color32::DARK_RED, error); }
            if self.materials_page && (conditions.is_err() || date.is_err()) {
                ui.label(RichText::new("Complete the temperatures and date in Conditions before saving.").color(MUTED));
            }
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() { action = Some(FormAction::Cancel); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(self.conditions().is_ok() && self.date().is_ok(), |ui| {
                        if primary_button(ui, "Save experiment").clicked() { action = Some(FormAction::Save); }
                    });
                    if !self.materials_page && ui.button("Next: materials").clicked() {
                        self.materials_page = true;
                        ctx.request_repaint();
                    }
                });
            });
        });
        action.or(changed.then_some(FormAction::Changed))
    }
}

fn material_card(
    ui: &mut egui::Ui,
    title: &str,
    material: &mut crate::model::Material,
    changed: &mut bool,
) {
    egui::Frame::new()
        .fill(PAPER)
        .stroke(Stroke::new(1.0, Color32::from_gray(217)))
        .corner_radius(8.0)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            *changed |= materials_view::material_input(ui, title, material);
        });
}

pub enum ExperimentAction {
    New,
    Open(String),
    PreviousLogs,
    Delete(String),
    RecentlyDeleted,
}

pub fn experiments(
    ui: &mut egui::Ui,
    experiments: &[CvtExperiment],
    has_previous_logs: bool,
) -> Option<ExperimentAction> {
    let mut action = None;
    egui::ScrollArea::vertical().id_salt("experiments-page").show(ui, |ui| {
        egui::Frame::new().inner_margin(26).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("CVT Experiments").size(28.0).color(INK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button(ui, "+  New experiment").clicked() { action = Some(ExperimentAction::New); }
                    if ui.button("Recently deleted").clicked() { action = Some(ExperimentAction::RecentlyDeleted); }
                    if has_previous_logs && ui.button("Previous project logs").clicked() { action = Some(ExperimentAction::PreviousLogs); }
                });
            });
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Temperature overview").size(18.0).strong())
                    .on_hover_text("Horizontal position is schematic, not distance or time. Lines join set-points rather than measured temperature profiles. ΔT = Source − Sink.");
                ui.label(RichText::new(format!("{} saved", experiments.len())).color(MUTED));
            });
            ui.add_space(8.0);
            let mut chart_overflows = false;
            egui::Frame::new().fill(PAPER).stroke(Stroke::new(1.0, Color32::from_gray(224)))
                .corner_radius(8.0).inner_margin(12).show(ui, |ui| {
                    let (overflows,open) = draw_chart(ui, experiments);
                    chart_overflows = overflows;
                    if let Some(id)=open { action = Some(ExperimentAction::Open(id)); }

                });
            ui.add_space(8.0);
            ui.label(RichText::new("Click an experiment bar or name to open its log.").color(TEAL));
            if chart_overflows {
                ui.label(RichText::new("Scroll the overview horizontally to see more experiments.").size(11.0).color(MUTED));
            }
            ui.add_space(22.0);
            if experiments.is_empty() {
                ui.label(RichText::new("No experiments yet").size(18.0).strong());
                ui.label("Use + New experiment to record your first source and sink temperatures.");
            } else {
                ui.label(RichText::new("Recorded experiments").size(18.0).strong());
                ui.label(RichText::new("All saved experiments · most recently recorded first").size(11.0).color(MUTED));
                ui.add_space(12.0);
                let name_width = (ui.available_width() * 0.25).clamp(120.0, 210.0);
                egui::ScrollArea::horizontal().id_salt("recorded-experiments-horizontal").show(ui,|ui| {
                egui::Grid::new("cvt-records").striped(true).min_col_width(85.0).spacing([12.0, 12.0]).show(ui, |ui| {
                    for title in ["Experiment", "Experiment date", "Source (°C)", "Sink (°C)", "ΔT (°C)", "Thermal character", "Actions"] { ui.strong(title); }
                    ui.end_row();
                    for experiment in experiments {
                        let response = ui.allocate_ui_with_layout(Vec2::new(name_width, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(egui::Label::new(RichText::new(&experiment.name).color(TEAL).underline()).truncate().sense(egui::Sense::click()))
                        }).inner
                            .on_hover_text(format!("{}\nRecorded {} UTC\nID: {}", experiment.name, experiment.created_at.format("%Y-%m-%d %H:%M"), experiment.id));
                        if response.clicked() {action=Some(ExperimentAction::Open(experiment.id.clone()));}
                        ui.label(experiment.details.experiment_date.map(|date| date.to_string()).unwrap_or_else(|| "Not specified".into())).on_hover_text(format!("Recorded {} UTC", experiment.created_at.format("%Y-%m-%d %H:%M")));
                        ui.label(temperature_text(experiment.conditions.source_c()));
                        ui.label(temperature_text(experiment.conditions.sink_c()));
                        ui.colored_label(TEAL, temperature_text(experiment.conditions.delta_c()));
                        ui.label(experiment.details.heat_effect.label());
                        if ui.button("Delete").clicked() { action=Some(ExperimentAction::Delete(experiment.id.clone())); }
                        ui.end_row();
                    }
                });
                });
            }
        });
    });
    action
}

fn draw_chart(ui: &mut egui::Ui, experiments: &[CvtExperiment]) -> (bool, Option<String>) {
    let scale = TemperatureScale::for_conditions(experiments.iter().map(|e| e.conditions));
    let height = 330.0;
    let mut overflows = false;
    let mut open = None;
    // Keep the temperature axis fixed while experiment groups scroll horizontally.
    ui.horizontal_top(|ui| {
        let (axis, _) = ui.allocate_exact_size(Vec2::new(72.0, height), egui::Sense::hover());
        let y = |rect: Rect, value: f64| {
            rect.top() + 44.0 + (1.0 - scale.fraction(value)) as f32 * 210.0
        };
        let painter = ui.painter_at(axis);
        painter.text(
            axis.left_top() + Vec2::new(0.0, 12.0),
            Align2::LEFT_CENTER,
            "T / °C",
            FontId::proportional(13.0),
            INK,
        );
        painter.line_segment(
            [
                Pos2::new(axis.right() - 2.0, axis.top() + 44.0),
                Pos2::new(axis.right() - 2.0, axis.top() + 254.0),
            ],
            Stroke::new(1.0, MUTED),
        );
        for tick in scale.ticks() {
            let point = Pos2::new(axis.right() - 9.0, y(axis, tick));
            painter.text(
                point,
                Align2::RIGHT_CENTER,
                temperature_text(tick),
                FontId::proportional(11.0),
                MUTED,
            );
        }
        egui::ScrollArea::horizontal()
            .id_salt("cvt-chart-horizontal")
            .show(ui, |ui| {
                let width = (experiments.len().max(1) as f32 * 238.0).max(ui.available_width());
                overflows = width > ui.available_width() + 1.0;
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                for tick in scale.ticks() {
                    let height = y(rect, tick);
                    painter.line_segment(
                        [
                            Pos2::new(rect.left(), height),
                            Pos2::new(rect.right(), height),
                        ],
                        Stroke::new(1.0, Color32::from_gray(225)),
                    );
                }
                let axis_y = rect.top() + 254.0;
                painter.line_segment(
                    [
                        Pos2::new(rect.left(), axis_y),
                        Pos2::new(rect.right(), axis_y),
                    ],
                    Stroke::new(1.0, MUTED),
                );
                if experiments.is_empty() {
                    painter.text(
                        Pos2::new(rect.center().x, rect.top() + 135.0),
                        Align2::CENTER_CENTER,
                        "Your experiments will appear here",
                        FontId::proportional(17.0),
                        MUTED,
                    );
                    painter.text(
                        Pos2::new(rect.center().x, axis_y + 26.0),
                        Align2::CENTER_CENTER,
                        "Schematic source / sink position",
                        FontId::proportional(12.0),
                        MUTED,
                    );
                }
                let slot = width / experiments.len().max(1) as f32;
                for (index, experiment) in experiments.iter().enumerate() {
                    let center = rect.left() + slot * (index as f32 + 0.5);
                    let source_x = center - slot * 0.28;
                    let sink_x = center + slot * 0.28;
                    let source = Pos2::new(source_x, y(rect, experiment.conditions.source_c()));
                    let sink = Pos2::new(sink_x, y(rect, experiment.conditions.sink_c()));
                    let short_name: String = experiment.name.chars().take(26).collect();
                    painter.text(
                        Pos2::new(center, rect.top() + 17.0),
                        Align2::CENTER_CENTER,
                        short_name,
                        FontId::proportional(13.0),
                        INK,
                    );
                    painter.line_segment([source, sink], Stroke::new(2.5, TEAL));
                    for (point, color, label, temperature) in [
                        (source, SOURCE, "Source", experiment.conditions.source_c()),
                        (sink, SINK, "Sink", experiment.conditions.sink_c()),
                    ] {
                        painter.line_segment(
                            [point, Pos2::new(point.x, axis_y)],
                            Stroke::new(1.0, Color32::from_gray(211)),
                        );
                        painter.circle_filled(point, 5.0, color);
                        painter.text(
                            point - Vec2::new(0.0, 13.0),
                            Align2::CENTER_BOTTOM,
                            format!("{} °C", temperature_text(temperature)),
                            FontId::proportional(12.0),
                            color,
                        );
                        painter.text(
                            Pos2::new(point.x, axis_y + 19.0),
                            Align2::CENTER_CENTER,
                            label,
                            FontId::proportional(11.0),
                            MUTED,
                        );
                    }
                    painter.text(
                        Pos2::new(center, axis_y + 49.0),
                        Align2::CENTER_CENTER,
                        format!(
                            "ΔT  {} °C",
                            temperature_text(experiment.conditions.delta_c())
                        ),
                        FontId::proportional(14.0),
                        TEAL,
                    );
                    let hover_rect = Rect::from_min_max(
                        Pos2::new(center - slot * 0.5, rect.top()),
                        Pos2::new(center + slot * 0.5, rect.bottom()),
                    );
                    let response=ui.interact(
                        hover_rect,
                        egui::Id::new(("cvt-profile", &experiment.id)),
                        egui::Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(format!(
                        "{}\nSource: {} °C\nSink: {} °C\nΔT (Source − Sink): {} °C\n{}\nClick to open experiment log",
                        experiment.name,
                        temperature_text(experiment.conditions.source_c()),
                        temperature_text(experiment.conditions.sink_c()),
                        temperature_text(experiment.conditions.delta_c()), experiment.details.heat_effect.label()
                    ));
                    if response.clicked() {open=Some(experiment.id.clone());}
                }
            });
    });
    (overflows, open)
}
