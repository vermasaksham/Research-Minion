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
    pub name: String,
    pub source: String,
    pub sink: String,
    pub details: ExperimentDetails,
    target: ElementTarget,
    ui_id: uuid::Uuid,
}

impl Default for ExperimentForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            source: String::new(),
            sink: String::new(),
            details: ExperimentDetails::default(),
            target: ElementTarget::default(),
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
        !self.name.is_empty()
            || !self.source.is_empty()
            || !self.sink.is_empty()
            || self.details != ExperimentDetails::default()
    }

    pub fn conditions(&self) -> Result<CvtConditions, TemperatureError> {
        CvtConditions::parse(&self.source, &self.sink)
    }

    pub fn show(&mut self, ctx: &egui::Context, error: Option<&str>) -> Option<FormAction> {
        let mut action = None;
        let mut changed = false;
        // Fresh forms get fresh scroll/collapse state; retained drafts keep theirs.
        egui::Modal::new(egui::Id::new(("new-cvt-experiment",self.ui_id))).show(ctx, |ui| {
            ui.set_width(860.0);
            ui.heading("New CVT experiment");
            ui.label(RichText::new("Record temperatures, materials and thermal character.").color(MUTED));
            ui.add_space(10.0);
            egui::ScrollArea::vertical().id_salt("experiment-form-body")
                .max_height((ctx.content_rect().height()-190.0).clamp(240.0,620.0)).show(ui,|ui| {
                ui.label("Experiment name (optional)");
                changed |= ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Leave blank to generate a label").desired_width(f32::INFINITY)).changed();
                ui.add_space(12.0);
                ui.columns(2,|columns| {
                    columns[0].strong("Source temperature (°C)");
                    changed |= columns[0].add(egui::TextEdit::singleline(&mut self.source).hint_text("e.g. 650.5").desired_width(f32::INFINITY)).changed();
                    columns[1].strong("Sink temperature (°C)");
                    changed |= columns[1].add(egui::TextEdit::singleline(&mut self.sink).hint_text("e.g. 600").desired_width(f32::INFINITY)).changed();
                });
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    match self.conditions() {
                        Ok(pair)=>{ui.label(RichText::new(format!("ΔT = {} °C",temperature_text(pair.delta_c()))).size(20.0).color(TEAL));}
                        Err(error) if !self.source.is_empty() && !self.sink.is_empty()=>{ui.colored_label(Color32::DARK_RED,error.to_string());}
                        Err(_)=>{ui.label(RichText::new("ΔT = Source − Sink").color(MUTED));}
                    }
                    ui.label("Thermal character:");
                    egui::ComboBox::from_id_salt("heat-effect").selected_text(self.details.heat_effect.label()).show_ui(ui,|ui| {
                        for effect in [HeatEffect::Unspecified,HeatEffect::Endothermic,HeatEffect::Exothermic] {
                            changed |= ui.selectable_value(&mut self.details.heat_effect,effect,effect.label()).changed();
                        }
                    });
                });
                ui.label(RichText::new("Endothermic absorbs heat; exothermic releases heat. Your assignment is not inferred from ΔT.").size(11.0).color(MUTED));
                ui.add_space(14.0);
                ui.columns(2,|columns| {
                    changed |= materials_view::material_input(&mut columns[0],"Precursor materials",&mut self.details.precursor);
                    changed |= materials_view::material_input(&mut columns[1],"Obtained material",&mut self.details.product);
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Suggestions recognise element symbols, formulas and common name parts. Confirm the elements you want; your typed text is always kept.").size(11.0).color(MUTED));
                ui.collapsing("Choose elements from the periodic table",|ui| {
                    changed |= materials_view::periodic_table(ui,&mut self.details,&mut self.target);
                });
                ui.add_space(12.0);
                ui.label("Transport agent (optional)");
                changed |= ui.add(egui::TextEdit::singleline(&mut self.details.transport_agent).hint_text("Formula or name, e.g. I2; leave blank if not used").desired_width(f32::INFINITY)).changed();
            });
            if let Some(error)=error {ui.colored_label(Color32::DARK_RED,error);}
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_enabled_ui(self.conditions().is_ok(),|ui| {
                    if primary_button(ui,"Save experiment").clicked(){action=Some(FormAction::Save);}
                });
                if ui.button("Cancel").clicked(){action=Some(FormAction::Cancel);}
            });
        });
        action.or(changed.then_some(FormAction::Changed))
    }
}

pub enum ExperimentAction {
    New,
    Open(String),
    PreviousLogs,
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
                ui.vertical(|ui| {
                    ui.label(RichText::new("EXPERIMENTS").size(11.0).strong().color(TEAL));
                    ui.add_space(4.0);
                    ui.heading(RichText::new("CVT Experiments").size(28.0).color(INK));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button(ui, "+  New experiment").clicked() { action = Some(ExperimentAction::New); }
                    if has_previous_logs && ui.button("Previous project logs").clicked() { action = Some(ExperimentAction::PreviousLogs); }
                });
            });
            ui.add_space(7.0);
            ui.label(RichText::new("Your source and sink temperatures, at a glance.").color(MUTED));
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Temperature overview").size(18.0).strong());
                ui.label(RichText::new(format!("{} saved", experiments.len())).color(MUTED));
            });
            ui.add_space(8.0);
            let mut chart_overflows = false;
            egui::Frame::new().fill(PAPER).stroke(Stroke::new(1.0, Color32::from_gray(224)))
                .corner_radius(8.0).inner_margin(12).show(ui, |ui| {
                    let (overflows,open) = draw_chart(ui, experiments);
                    chart_overflows = overflows;
                    if let Some(id)=open { action = Some(ExperimentAction::Open(id)); }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.colored_label(SOURCE, "Source");
                        ui.colored_label(SINK, "Sink");
                        ui.label(RichText::new("ΔT = Source − Sink").color(MUTED));
                    });
                });
            ui.add_space(8.0);
            ui.label(RichText::new("Click an experiment bar or name to open its log.").color(TEAL));
            ui.label(RichText::new("Horizontal position is schematic, not distance or time. Lines connect set-points; they are not measured temperature profiles.").size(11.0).color(MUTED));
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
                let name_width = (ui.available_width() * 0.35).clamp(140.0, 240.0);
                egui::Grid::new("cvt-records").striped(true).min_col_width(85.0).spacing([12.0, 12.0]).show(ui, |ui| {
                    for title in ["Experiment", "Source (°C)", "Sink (°C)", "ΔT (°C)", "Thermal character"] { ui.strong(title); }
                    ui.end_row();
                    for experiment in experiments {
                        let response = ui.allocate_ui_with_layout(Vec2::new(name_width, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(egui::Label::new(RichText::new(&experiment.name).color(TEAL).underline()).truncate().sense(egui::Sense::click()))
                        }).inner
                            .on_hover_text(format!("{}\nRecorded {} UTC\nID: {}", experiment.name, experiment.created_at.format("%Y-%m-%d %H:%M"), experiment.id));
                        if response.clicked() {action=Some(ExperimentAction::Open(experiment.id.clone()));}
                        ui.label(temperature_text(experiment.conditions.source_c()));
                        ui.label(temperature_text(experiment.conditions.sink_c()));
                        ui.colored_label(TEAL, temperature_text(experiment.conditions.delta_c()));
                        ui.label(experiment.details.heat_effect.label());
                        ui.end_row();
                    }
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

pub fn tools(ui: &mut egui::Ui) {
    egui::Frame::new().inner_margin(28).show(ui, |ui| {
        ui.label(
            RichText::new("SCIENTIFIC UTILITIES")
                .size(11.0)
                .strong()
                .color(TEAL),
        );
        ui.heading(RichText::new("Tools").size(28.0));
        ui.add_space(8.0);
        ui.label(RichText::new("Your scientific tools will live here.").color(MUTED));
        ui.add_space(26.0);
        egui::Frame::new()
            .fill(PAPER)
            .inner_margin(22)
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.heading("Sb₂Se₃ Peak Explorer");
                ui.add_space(8.0);
                ui.label(RichText::new("Planned · not available in this build").color(TEAL));
                ui.label("EDX reference markers and calculated XRD reflection positions.");
                ui.label(RichText::new("Planned as a local, offline tool.").color(MUTED));
            });
    });
}
