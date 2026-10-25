use crate::{
    app::{MUTED, PAPER, TEAL, primary_button},
    model::{CvtExperiment, EntryDraft},
    peak::{self, Analysis, OrthorhombicCell},
    store::Store,
};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, RichText, Stroke, Vec2};

pub struct PeakWorkspace {
    open: bool,
    a: String,
    b: String,
    c: String,
    wavelength: String,
    indices: String,
    material: String,
    setting: String,
    source: String,
    interpretation: String,
    target: Option<String>,
    event_date: String,
    notice: Option<String>,
    bragg_d: String,
    bragg_angle: String,
}

impl Default for PeakWorkspace {
    fn default() -> Self {
        Self {
            open: false,
            a: String::new(),
            b: String::new(),
            c: String::new(),
            wavelength: "1.5406".into(),
            indices: "1 0 0\n0 1 0\n0 0 1".into(),
            material: String::new(),
            setting: String::new(),
            source: String::new(),
            interpretation: String::new(),
            target: None,
            event_date: chrono::Local::now().date_naive().to_string(),
            notice: None,
            bragg_d: String::new(),
            bragg_angle: String::new(),
        }
    }
}

fn number(text: &str, label: &str) -> Result<f64, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("Enter {label} as a number."))
}

impl PeakWorkspace {
    fn analysis(&self) -> Result<Analysis, String> {
        let cell = OrthorhombicCell::new(
            number(&self.a, "a (Å)")?,
            number(&self.b, "b (Å)")?,
            number(&self.c, "c (Å)")?,
        )
        .map_err(|error| error.to_string())?;
        let wavelength_a = number(&self.wavelength, "wavelength (Å)")?;
        let indices = peak::parse_reflections(&self.indices).map_err(|error| error.to_string())?;
        let results =
            peak::calculate(cell, wavelength_a, &indices).map_err(|error| error.to_string())?;
        Ok(Analysis {
            cell,
            wavelength_a,
            results,
            material: self.material.clone(),
            setting: self.setting.clone(),
            source: self.source.clone(),
            interpretation: self.interpretation.clone(),
        })
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        store: Option<&Store>,
        experiments: &[CvtExperiment],
    ) {
        egui::ScrollArea::vertical().id_salt("tools-page").show(ui,|ui| {
            egui::Frame::new().inner_margin(24).show(ui,|ui| {
                if !self.open {
                    ui.heading(RichText::new("Tools").size(28.0));
                    ui.add_space(22.0);
                    egui::Frame::new().fill(PAPER).inner_margin(20).corner_radius(8.0).show(ui,|ui| {
                        ui.heading("Peak Explorer");
                        ui.label("Offline XRD positions from your orthorhombic cell and chosen reflections.");
                        ui.add_space(12.0);
                        if primary_button(ui,"Open Peak Explorer").clicked(){self.open=true;ui.ctx().request_repaint();}
                    });
                    return;
                }
                if primary_button(ui,"Back to tools").clicked(){self.open=false;ui.ctx().request_repaint();}
                ui.add_space(14.0);
                ui.heading("Peak Explorer · XRD");
                ui.label(RichText::new("First-order Bragg positions · user-entered orthorhombic cell").color(TEAL));
                ui.label("Enter your own cell parameters and their source. The original Sb₂Se₃ reference catalog and EDX markers are not bundled yet.");
                ui.add_space(16.0);
                ui.label("Material or phase label (optional)");
                ui.add(egui::TextEdit::singleline(&mut self.material).desired_width(f32::INFINITY).hint_text("Your sample/phase description"));
                ui.horizontal_wrapped(|ui| {
                    for (label,value) in [("a (Å)",&mut self.a),("b (Å)",&mut self.b),("c (Å)",&mut self.c),("λ (Å)",&mut self.wavelength)] {
                        ui.label(label); ui.add(egui::TextEdit::singleline(value).desired_width(82.0));
                    }
                });
                ui.label(RichText::new("α = β = γ = 90°. λ starts at 1.5406 Å; enter the wavelength and convention for your source.").size(11.0).color(MUTED));
                ui.label("Axis / structure setting (optional; axes used exactly as entered)");
                ui.add(egui::TextEdit::singleline(&mut self.setting).desired_width(f32::INFINITY).hint_text("e.g. setting and axis order from your source; no automatic conversion"));
                ui.label("Cell source / provenance (optional, recommended)");
                ui.add(egui::TextEdit::singleline(&mut self.source).desired_width(f32::INFINITY).hint_text("CIF identifier, publication, measurement, or calculation source"));
                ui.add_space(12.0);
                ui.label("Selected reflections · one h k l per line (maximum 500)");
                egui::ScrollArea::vertical().id_salt("hkl-input").max_height(130.0).show(ui,|ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.indices).font(egui::TextStyle::Monospace).desired_rows(4).desired_width(f32::INFINITY));
                });
                let analysis=self.analysis();
                ui.add_space(14.0);
                match &analysis {
                    Ok(analysis)=> {
                        position_plot(ui,analysis);
                        ui.label(RichText::new("Equal-height markers show calculated positions only. They do not represent intensity.").color(MUTED).size(11.0));
                        egui::ScrollArea::vertical().id_salt("peak-results").max_height(220.0).show(ui,|ui| {
                            egui::Grid::new("peak-result-table").striped(true).spacing([25.0,7.0]).show(ui,|ui| {
                                for title in ["h k l","d (Å)","2θ (degrees)"] {ui.strong(title);} ui.end_row();
                                for row in &analysis.results {
                                    ui.monospace(format!("{} {} {}",row.hkl[0],row.hkl[1],row.hkl[2]));
                                    ui.label(format!("{:.8}",row.d_a));
                                    ui.label(row.two_theta.map(|angle|format!("{angle:.6}")).unwrap_or_else(||"No Bragg solution (λ > 2d)".into()));
                                    ui.end_row();
                                }
                            });
                        });
                    }
                    Err(error)=>{ui.label(RichText::new(error).color(Color32::DARK_RED));}
                }
                ui.add_space(10.0);
                ui.label("Positions only. No symmetry-absence checks, intensities, phase fractions, phase identification, or Pbnm/Pnma conversion.");
                ui.add_space(18.0);
                ui.collapsing("Bragg calculator · n = 1",|ui| {
                    ui.label("Uses the wavelength above. EDX energies (keV) are a different quantity from XRD angles.");
                    ui.horizontal_wrapped(|ui| {
                        ui.label("d (Å)"); ui.add(egui::TextEdit::singleline(&mut self.bragg_d).desired_width(110.0));
                        let result=number(&self.wavelength,"wavelength").and_then(|wavelength| {
                            peak::two_theta_deg(wavelength,number(&self.bragg_d,"d")?).map_err(|error|error.to_string())
                        });
                        ui.label(match result {Ok(Some(angle))=>format!("→ 2θ = {angle:.6}°"),Ok(None)=>"No first-order solution (λ > 2d)".into(),Err(error)=>error});
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label("2θ (°)"); ui.add(egui::TextEdit::singleline(&mut self.bragg_angle).desired_width(110.0));
                        let result=number(&self.wavelength,"wavelength").and_then(|wavelength| {
                            peak::d_spacing_a(wavelength,number(&self.bragg_angle,"2θ")?).map_err(|error|error.to_string())
                        });
                        ui.label(match result {Ok(d)=>format!("→ d = {d:.8} Å"),Err(error)=>error});
                    });
                });
                ui.separator();
                ui.heading("Save analysis to an experiment log");
                ui.label("Saves a new entry containing inputs, provenance, calculation version, results and interpretation.");
                let chosen=experiments.iter().find(|experiment|Some(&experiment.id)==self.target.as_ref());
                egui::ComboBox::from_id_salt("analysis-target").selected_text(chosen.map_or("Choose an experiment",|experiment|experiment.name.as_str())).show_ui(ui,|ui| {
                    for experiment in experiments {ui.selectable_value(&mut self.target,Some(experiment.id.clone()),format!("{} · {}",experiment.name,experiment.id.chars().take(8).collect::<String>()));}
                });
                ui.horizontal(|ui| {
                    ui.label("Analysis event date");
                    ui.add(egui::TextEdit::singleline(&mut self.event_date).desired_width(112.0).hint_text("YYYY-MM-DD"));
                });
                ui.label("Your interpretation (optional)");
                ui.add(egui::TextEdit::multiline(&mut self.interpretation).desired_rows(3).desired_width(f32::INFINITY));
                if experiments.is_empty(){ui.label("Create a CVT experiment first to save a linked analysis.");}
                if ui.add_enabled(analysis.is_ok() && chosen.is_some(),egui::Button::new("Save analysis to log")).clicked() {
                    // Recalculate after interpretation changes this frame; freeze the exact snapshot.
                    let result=self.analysis().and_then(|analysis| {
                        let event_date=chrono::NaiveDate::parse_from_str(self.event_date.trim(),"%Y-%m-%d").ok().filter(|date|date.to_string()==self.event_date.trim()).ok_or_else(||"Enter a valid analysis date in YYYY-MM-DD format.".to_string())?;
                        let target=chosen.ok_or("Choose an experiment.")?;
                        let store=store.ok_or("Workspace is unavailable.")?;
                        store.create_entry(&EntryDraft {
                            project_id:target.log_project_id.clone(),
                            title:format!("Peak Explorer XRD · {}",if analysis.material.trim().is_empty(){target.name.as_str()}else{analysis.material.trim()}),
                            body:analysis.report(),event_date,
                        }).map_err(|error|error.to_string())?;
                        Ok(format!("Saved a new analysis entry in {}. Click its experiment bar to open the log.",target.name))
                    });
                    self.notice=Some(result.unwrap_or_else(|error|format!("Could not save: {error}")));
                }
                if let Some(notice)=&self.notice {ui.label(RichText::new(notice).color(TEAL));}
                ui.add_space(12.0);
                ui.label(RichText::new("Tool inputs are kept while this app is open. Save an analysis entry before closing to retain a snapshot.").size(11.0).color(MUTED));
            });
        });
    }
}

fn position_plot(ui: &mut egui::Ui, analysis: &Analysis) {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().max(240.0), 150.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let left = rect.left() + 20.0;
    let width = (rect.width() - 45.0).max(1.0);
    let bottom = rect.bottom() - 30.0;
    let x = |angle: f64| left + width * (angle / 180.0) as f32;
    painter.line_segment(
        [Pos2::new(left, bottom), Pos2::new(left + width, bottom)],
        Stroke::new(1.0, MUTED),
    );
    for tick in (0..=180).step_by(30) {
        let pos = Pos2::new(x(f64::from(tick)), bottom);
        painter.text(
            pos + Vec2::new(0.0, 7.0),
            Align2::CENTER_TOP,
            tick.to_string(),
            FontId::proportional(11.0),
            MUTED,
        );
    }
    for row in &analysis.results {
        if let Some(angle) = row.two_theta {
            painter.line_segment(
                [
                    Pos2::new(x(angle), bottom),
                    Pos2::new(x(angle), rect.top() + 25.0),
                ],
                Stroke::new(1.5, TEAL),
            );
        }
    }
    if let Some(pointer) = response.hover_pos() {
        let nearby: Vec<_> = analysis
            .results
            .iter()
            .filter_map(|row| {
                let angle = row.two_theta?;
                ((x(angle) - pointer.x).abs() <= 6.0).then(|| {
                    format!(
                        "{} {} {} · d = {:.8} Å · 2θ = {:.6}°",
                        row.hkl[0], row.hkl[1], row.hkl[2], row.d_a, angle
                    )
                })
            })
            .collect();
        if !nearby.is_empty() {
            response.on_hover_text(nearby.join("\n"));
        }
    }
    painter.text(
        rect.left_top(),
        Align2::LEFT_TOP,
        "XRD 2θ / degrees · positions only",
        FontId::proportional(12.0),
        TEAL,
    );
}
