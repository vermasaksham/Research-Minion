use crate::{
    app::{INK, MUTED, PAPER, TEAL},
    chemistry::{ELEMENTS, element, suggest_elements, symbols, table_position},
    model::{CvtExperiment, ExperimentDetails, Material},
};
use eframe::egui::{self, Color32, RichText, Vec2};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ElementTarget {
    #[default]
    Precursor,
    Product,
}

pub fn material_input(ui: &mut egui::Ui, title: &str, material: &mut Material) -> bool {
    let mut changed = false;
    ui.label(RichText::new(title).size(17.0).strong().color(INK));
    ui.add_space(3.0);
    changed |= ui
        .add(
            egui::TextEdit::singleline(&mut material.text)
                .hint_text("Formula or chemical name")
                .margin(egui::vec2(10.0, 9.0))
                .desired_width(f32::INFINITY),
        )
        .on_hover_text(
            "Your text is saved as entered. Suggestions are added only when you select them.",
        )
        .changed();
    let suggestions: Vec<_> = suggest_elements(&material.text)
        .into_iter()
        .filter(|number| !material.elements.contains(number))
        .collect();
    if !suggestions.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new("Suggestions").size(13.0).color(MUTED));
        ui.horizontal_wrapped(|ui| {
            for number in &suggestions {
                let e = element(*number).expect("suggestions use the element catalog");
                if ui
                    .button(format!("+ {}", e.symbol))
                    .on_hover_text(e.name)
                    .clicked()
                {
                    material.elements.insert(*number);
                    changed = true;
                }
            }
            if ui.button("Add all").clicked() {
                material.elements.extend(suggestions);
                changed = true;
            }
        });
    }
    ui.add_space(6.0);
    ui.label(
        RichText::new("Selected elements")
            .size(13.0)
            .strong()
            .color(INK),
    );
    let selected: Vec<_> = material.elements.iter().copied().collect();
    ui.horizontal_wrapped(|ui| {
        if selected.is_empty() {
            ui.label(RichText::new("None selected").size(13.0).color(MUTED));
        }
        for number in selected {
            if let Some(e) = element(number)
                && ui
                    .button(format!("{} ×", e.symbol))
                    .on_hover_text(format!("Remove {}", e.name))
                    .clicked()
            {
                material.elements.remove(&number);
                changed = true;
            }
        }
    });
    changed
}

pub fn periodic_table(
    ui: &mut egui::Ui,
    details: &mut ExperimentDetails,
    target: &mut ElementTarget,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Select elements for:");
        ui.selectable_value(target, ElementTarget::Precursor, "Precursor");
        ui.selectable_value(target, ElementTarget::Product, "Obtained material");
    });
    ui.label(
        RichText::new("Click an element to select or deselect it. Hover for its name.")
            .size(11.0)
            .color(MUTED),
    );
    let material = match target {
        ElementTarget::Precursor => &mut details.precursor,
        ElementTarget::Product => &mut details.product,
    };
    ui.label(
        RichText::new(format!(
            "Selected: {}",
            if material.elements.is_empty() {
                "None".into()
            } else {
                symbols(&material.elements)
            }
        ))
        .color(TEAL),
    );
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::ZERO;
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);
        let cell = Vec2::new(34.0, 35.0);
        ui.horizontal(|ui| {
            ui.allocate_exact_size(Vec2::new(20.0, 16.0), egui::Sense::hover());
            for group in 1..=18 {
                ui.add_sized(
                    [34.0, 16.0],
                    egui::Label::new(RichText::new(group.to_string()).size(10.0).color(MUTED)),
                );
            }
        });
        for row in [0, 1, 2, 3, 4, 5, 6, 8, 9] {
            if row == 8 {
                ui.add_space(10.0);
            }
            ui.horizontal(|ui| {
                ui.add_sized(
                    [20.0, 35.0],
                    egui::Label::new(
                        RichText::new(if row < 7 {
                            (row + 1).to_string()
                        } else {
                            String::new()
                        })
                        .size(10.0)
                        .color(MUTED),
                    ),
                );
                for column in 0..18 {
                    if let Some(e) = ELEMENTS
                        .iter()
                        .find(|e| table_position(e.number) == Some((row, column)))
                    {
                        let selected = material.elements.contains(&e.number);
                        let label = RichText::new(format!("{}\n{}", e.number, e.symbol))
                            .size(12.0)
                            .color(if selected { Color32::WHITE } else { INK });
                        if ui
                            .add_sized(
                                cell,
                                egui::Button::new(label).fill(if selected { TEAL } else { PAPER }),
                            )
                            .on_hover_text(format!(
                                "{} · {} · atomic number {}",
                                e.symbol, e.name, e.number
                            ))
                            .clicked()
                        {
                            if selected {
                                material.elements.remove(&e.number);
                            } else {
                                material.elements.insert(e.number);
                            }
                            changed = true;
                        }
                    } else if column == 2 && (row == 5 || row == 6) {
                        ui.add_sized(
                            cell,
                            egui::Label::new(
                                RichText::new(if row == 5 { "57-71" } else { "89-103" })
                                    .size(10.0)
                                    .color(MUTED),
                            ),
                        );
                    } else {
                        ui.allocate_exact_size(cell, egui::Sense::hover());
                    }
                }
            });
        }
    });
    changed
}

pub fn experiment_summary(ui: &mut egui::Ui, experiment: &CvtExperiment) {
    ui.label(format!(
        "Experiment date: {}",
        experiment
            .details
            .experiment_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "Not specified".into()),
    ));
    ui.horizontal_wrapped(|ui| {
        for (label, value) in [
            ("Source", experiment.conditions.source_c()),
            ("Sink", experiment.conditions.sink_c()),
            ("ΔT", experiment.conditions.delta_c()),
        ] {
            ui.label(
                RichText::new(format!(
                    "{label} {} °C",
                    crate::cvt::temperature_text(value)
                ))
                .color(if label == "ΔT" { TEAL } else { INK }),
            );
        }
        ui.label(format!(
            "Thermal character: {}",
            experiment.details.heat_effect.label()
        ))
        .on_hover_text("Researcher-assigned; not inferred from ΔT.");
    });
    ui.add_space(5.0);
    ui.collapsing("Materials and transport agent", |ui| {
        for (name, material) in [
            ("Precursor", &experiment.details.precursor),
            ("Obtained material", &experiment.details.product),
        ] {
            ui.strong(name);
            ui.label(if material.text.trim().is_empty() {
                "No manual description"
            } else {
                &material.text
            });
            ui.label(format!(
                "Selected elements: {}",
                if material.elements.is_empty() {
                    "None recorded".into()
                } else {
                    symbols(&material.elements)
                }
            ));
        }
        ui.label(format!(
            "Transport agent: {}",
            if experiment.details.transport_agent.trim().is_empty() {
                "Not recorded"
            } else {
                &experiment.details.transport_agent
            }
        ));
        ui.label(
            RichText::new("Thermal character is researcher-assigned, not inferred from ΔT.")
                .size(11.0)
                .color(MUTED),
        );
    });
}
