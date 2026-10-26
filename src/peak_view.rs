use crate::{
    app::{INK, MUTED, PAPER, TEAL, primary_button},
    manual_peak_view::ManualWorkspace,
    model::{CvtExperiment, EntryDraft},
    peak_plot::{self, Marker},
    peak_reference::{
        self as reference, Catalog, DiffractionPosition, EmissionLine, ExplorerParameters,
        ReferenceSnapshot,
    },
    store::Store,
};
use eframe::egui::{self, Color32, RichText};

const PURPLE: Color32 = Color32::from_rgb(113, 83, 190);
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum View {
    #[default]
    Compare,
    Edx,
    Xrd,
    Guide,
    Manual,
}
pub struct PeakWorkspace {
    open: bool,
    view: View,
    parameters: ExplorerParameters,
    radiation: String,
    custom_lambda: String,
    selected_edx: Option<String>,
    selected_xrd: Option<String>,
    target: Option<String>,
    event_date: String,
    interpretation: String,
    notice: Option<String>,
    manual: ManualWorkspace,
}
impl Default for PeakWorkspace {
    fn default() -> Self {
        Self {
            open: false,
            view: View::Compare,
            parameters: ExplorerParameters::default(),
            radiation: "cu".into(),
            custom_lambda: "1.5406".into(),
            selected_edx: Some("Sb-L3M5".into()),
            selected_xrd: Some("sb2se3-240".into()),
            target: None,
            event_date: chrono::Local::now().date_naive().to_string(),
            interpretation: String::new(),
            notice: None,
            manual: ManualWorkspace::default(),
        }
    }
}
impl PeakWorkspace {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        store: Option<&Store>,
        experiments: &[CvtExperiment],
    ) {
        egui::ScrollArea::vertical().id_salt("peak-page").show(ui,|ui| {
            egui::Frame::new().inner_margin(20).show(ui,|ui| {
                if !self.open {
                    ui.heading(RichText::new("Scientific Utilities").size(28.));ui.add_space(18.);
                    card(ui,|ui| {ui.heading("Peak Explorer");ui.label("EDX characteristic energies and XRD crystal reflections — independent quantities, cited references.");
                        if primary_button(ui,"Open Peak Explorer").clicked(){self.open=true;}});return;
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Back to Scientific Utilities").clicked(){self.open=false;}
                    if ui.button("Reset reference selections").clicked(){self.parameters=ExplorerParameters::default();self.radiation="cu".into();self.selected_edx=Some("Sb-L3M5".into());self.selected_xrd=Some("sb2se3-240".into());}
                });ui.add_space(12.);
                ui.heading(RichText::new("Sb₂Se₃ Peak Explorer").size(28.).color(INK));
                ui.label(RichText::new("EDX photon energy ≠ XRD scattering angle").size(19.).color(PURPLE));
                ui.label("Elemental emission and crystal diffraction answer different questions. References and calculations run locally.");
                ui.add_space(12.);ui.horizontal_wrapped(|ui| {
                    for (view,label) in [(View::Compare,"Side by side"),(View::Edx,"EDX energies"),(View::Xrd,"XRD reflections"),(View::Guide,"Lab guide & sources"),(View::Manual,"Manual XRD")] {ui.selectable_value(&mut self.view,view,label);}
                });ui.separator();
                if self.view==View::Manual {self.manual.show(ui,store,experiments);return;}
                let catalog=match reference::catalog(){Ok(c)=>c,Err(e)=>{ui.colored_label(Color32::DARK_RED,e.to_string());return;}};
                self.selections(ui,catalog);ui.add_space(12.);
                match self.view {
                    View::Compare=>{
                        if ui.available_width()>=900. {ui.columns(2,|cols|{self.edx_card(&mut cols[0],catalog,false);self.xrd_card(&mut cols[1],catalog,false);});}
                        else {self.edx_card(ui,catalog,false);ui.add_space(14.);self.xrd_card(ui,catalog,false);}
                        comparison(ui,catalog);
                    },
                    View::Edx=>self.edx_card(ui,catalog,true),View::Xrd=>self.xrd_card(ui,catalog,true),
                    View::Guide=>guide(ui,catalog),View::Manual=>unreachable!(),
                }
                self.save_export(ui,catalog,store,experiments);
            });
        });
    }
    fn selections(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Quick presets");
                for (id, label, els) in [
                    ("sb2se3", "Sb₂Se₃", vec!["Sb", "Se"]),
                    ("sb", "Sb", vec!["Sb"]),
                    ("se", "Se", vec!["Se"]),
                ] {
                    if ui.button(label).clicked() {
                        self.parameters.elements = els.into_iter().map(String::from).collect();
                        self.parameters.phases = vec![id.into()];
                    }
                }
                if ui.button("Sb + Se + I (EDX)").clicked() {
                    self.parameters.elements = vec!["Sb".into(), "Se".into(), "I".into()];
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(PURPLE, "EDX elements");
                for el in &catalog.elements {
                    let mut selected = self.parameters.elements.contains(&el.symbol);
                    if ui
                        .checkbox(&mut selected, &el.symbol)
                        .on_hover_text(format!("{} · atomic number {}", el.name, el.atomic_number))
                        .changed()
                    {
                        toggle(&mut self.parameters.elements, &el.symbol, selected);
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(TEAL, "XRD phases");
                for phase in &catalog.phases {
                    let mut selected = self.parameters.phases.contains(&phase.id);
                    if ui
                        .checkbox(&mut selected, &phase.label)
                        .on_hover_text(format!(
                            "{} · {} · COD {}",
                            phase.name, phase.space_group, phase.cod
                        ))
                        .changed()
                    {
                        toggle(&mut self.parameters.phases, &phase.id, selected);
                    }
                }
            });
            ui.label(RichText::new("Selections are independent. Selecting an element does not establish a crystalline phase.").size(11.).color(MUTED));
        });
    }
    fn lines<'a>(&self, catalog: &'a Catalog) -> Vec<&'a EmissionLine> {
        catalog
            .edx
            .iter()
            .filter(|l| {
                self.parameters.elements.contains(&l.element)
                    && self
                        .parameters
                        .family
                        .as_ref()
                        .is_none_or(|f| f == &l.family)
            })
            .collect()
    }
    fn wavelength(&self, catalog: &Catalog) -> Result<f64, String> {
        if self.radiation == "custom" {
            let v: f64 = self
                .custom_lambda
                .trim()
                .parse()
                .map_err(|_| "Enter a valid custom wavelength in Å.")?;
            if !v.is_finite() || v <= 0. {
                return Err("Custom wavelength must be finite and greater than zero.".into());
            }
            Ok(v)
        } else {
            catalog
                .radiation
                .iter()
                .find(|r| r.id == self.radiation)
                .map(|r| r.wavelength_a)
                .ok_or_else(|| "Unknown radiation reference.".into())
        }
    }
    fn radiation_label(&self, catalog: &Catalog) -> String {
        catalog
            .radiation
            .iter()
            .find(|r| r.id == self.radiation)
            .map_or_else(
                || "Custom wavelength (user entered)".into(),
                |r| format!("{} · NIST vacuum wavelength", r.label),
            )
    }
    fn rows(&self, catalog: &Catalog) -> Result<Vec<DiffractionPosition>, String> {
        reference::diffraction_positions(
            catalog,
            &self.parameters.phases,
            self.wavelength(catalog)?,
            self.parameters.pnma,
        )
        .map_err(|e| e.to_string())
    }
    fn edx_card(&mut self, ui: &mut egui::Ui, catalog: &Catalog, full: bool) {
        card(ui, |ui| {
            ui.colored_label(PURPLE, "EDX / EDS");
            ui.heading("Which elements are present?");
            ui.label("Electronic transitions -> emitted photon energy");
            ui.horizontal_wrapped(|ui| {
                ui.label("Window");
                egui::ComboBox::from_id_salt("edx-window")
                    .selected_text(format!("0–{} keV", self.parameters.energy_max_kev))
                    .show_ui(ui, |ui| {
                        for max in [5., 15., 35., 100.] {
                            ui.selectable_value(
                                &mut self.parameters.energy_max_kev,
                                max,
                                format!("0–{max} keV"),
                            );
                        }
                    });
                ui.label("Beam");
                ui.add(
                    egui::DragValue::new(&mut self.parameters.beam_kv)
                        .range(0.1..=150.)
                        .speed(0.1)
                        .suffix(" kV"),
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Line family");
                ui.selectable_value(&mut self.parameters.family, None, "All");
                for f in ["K", "L"] {
                    ui.selectable_value(&mut self.parameters.family, Some(f.into()), f);
                }
            });
            let lines = self.lines(catalog);
            if !lines
                .iter()
                .any(|l| Some(&l.id) == self.selected_edx.as_ref())
            {
                self.selected_edx = lines.first().map(|l| l.id.clone());
            }
            let lanes: Vec<_> = catalog
                .elements
                .iter()
                .filter(|e| self.parameters.elements.contains(&e.symbol))
                .map(|e| e.symbol.clone())
                .collect();
            let markers: Vec<_> = lines
                .iter()
                .map(|l| Marker {
                    id: l.id.clone(),
                    lane: l.element.clone(),
                    value: l.energy_kev(),
                    title: format!(
                        "{} {} · {} keV\n{}",
                        l.element,
                        l.label,
                        l.energy_text_kev(),
                        l.transition
                    ),
                    enabled: reference::above_shell_edge(self.parameters.beam_kv, l.edge_ev)
                        .unwrap_or(false),
                })
                .collect();
            if let Some(id) = peak_plot::plot(
                ui,
                &markers,
                &lanes,
                0.,
                self.parameters.energy_max_kev,
                "Emitted photon energy / keV",
                self.selected_edx.as_deref(),
                PURPLE,
            ) {
                self.selected_edx = Some(id);
            }
            ui.label(RichText::new("Equal-height reference sticks; no intensity scale. Pale dashed sticks: beam at/below the shell edge.").size(11.).color(MUTED));
            ui.label(format!(
                "{} markers · {} beyond plot window",
                lines.len(),
                lines
                    .iter()
                    .filter(|l| l.energy_kev() > self.parameters.energy_max_kev)
                    .count()
            ));
            if let Some(l) = lines
                .iter()
                .find(|l| Some(&l.id) == self.selected_edx.as_ref())
            {
                ui.separator();
                ui.heading(format!("{} {}", l.element, l.label));
                ui.label(
                    RichText::new(format!("{} keV", l.energy_text_kev()))
                        .size(22.)
                        .color(PURPLE),
                );
                ui.label(format!(
                    "{} · direct experimental {} ± {} eV",
                    l.transition, l.reported_energy_ev, l.reported_uncertainty_ev
                ));
                if !l.mass_number.is_empty() {
                    ui.label(format!("NIST mass number A = {}", l.mass_number));
                }
                ui.hyperlink_to("NIST reference table", &l.source);
                let above = reference::above_shell_edge(self.parameters.beam_kv, l.edge_ev)
                    .unwrap_or(false);
                ui.label(format!(
                    "{} {} shell edge: {} ± {} eV",
                    if above { "Above" } else { "At/below" },
                    l.shell,
                    l.edge_reported_ev,
                    l.edge_reported_uncertainty_ev
                ));
                ui.label(if above {
                    "Direct ionisation is energetically possible; useful counts are not guaranteed."
                } else {
                    "Direct electron ionisation of this shell is not enabled at this beam energy."
                });
                if l.energy_kev() > self.parameters.energy_max_kev {
                    ui.colored_label(PURPLE, "Selected line is outside the plot window.");
                }
                if l.blend {
                    ui.label(&l.note);
                }
            } else {
                ui.label("No direct experimental lines for this element/family selection.");
            }
            let mut table_lines = lines.clone();
            if !full {
                table_lines.retain(|l| {
                    l.energy_kev() <= self.parameters.energy_max_kev
                        || Some(&l.id) == self.selected_edx.as_ref()
                });
                table_lines.sort_by(|a, b| {
                    (Some(&a.id) != self.selected_edx.as_ref())
                        .cmp(&(Some(&b.id) != self.selected_edx.as_ref()))
                        .then_with(|| a.energy_ev.total_cmp(&b.energy_ev))
                });
            }
            egui::ScrollArea::both()
                .id_salt("edx-table")
                .max_height(if full { 280. } else { 125. })
                .min_scrolled_height(if full { 240. } else { 125. })
                .show(ui, |ui| {
                    egui::Grid::new("edx-grid")
                        .striped(true)
                        .spacing([16., 6.])
                        .show(ui, |ui| {
                            ui.strong("Line");
                            ui.strong("E (keV)");
                            if full {
                                ui.strong("Transition");
                                ui.strong("Uncertainty (eV)");
                            }
                            ui.end_row();
                            for l in table_lines.iter().take(if full { usize::MAX } else { 5 }) {
                                if ui
                                    .selectable_label(
                                        Some(&l.id) == self.selected_edx.as_ref(),
                                        format!("{} {}", l.element, l.label),
                                    )
                                    .clicked()
                                {
                                    self.selected_edx = Some(l.id.clone());
                                }
                                ui.label(l.energy_text_kev());
                                if full {
                                    ui.label(&l.transition);
                                    ui.label(format!("±{}", l.reported_uncertainty_ev));
                                }
                                ui.end_row();
                            }
                        });
                });
            if !full && ui.link("Inspect all selected EDX references ->").clicked() {
                self.view = View::Edx;
            }
        });
    }
    fn xrd_card(&mut self, ui: &mut egui::Ui, catalog: &Catalog, full: bool) {
        card(ui, |ui| {
            ui.colored_label(TEAL, "XRD / POWDER DIFFRACTION");
            ui.heading("Which crystal structure?");
            ui.label("Periodic atomic arrangement -> diffraction angle");
            ui.horizontal_wrapped(|ui| {
                ui.label("Radiation");
                egui::ComboBox::from_id_salt("reference-radiation")
                    .selected_text(if self.radiation == "custom" {
                        "Custom λ"
                    } else {
                        catalog
                            .radiation
                            .iter()
                            .find(|r| r.id == self.radiation)
                            .map_or("Choose", |r| r.label.as_str())
                    })
                    .show_ui(ui, |ui| {
                        for r in &catalog.radiation {
                            ui.selectable_value(&mut self.radiation, r.id.clone(), &r.label);
                        }
                        ui.selectable_value(&mut self.radiation, "custom".into(), "Custom λ");
                    });
                if self.radiation == "custom" {
                    ui.add(egui::TextEdit::singleline(&mut self.custom_lambda).desired_width(88.));
                    ui.label("Å");
                }
            });
            if let Ok(w) = self.wavelength(catalog) {
                ui.label(format!("λ = {w:.8} Å · Bragg order n = 1"));
            }
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.parameters.pnma, "Sb₂Se₃: mapped Pnma indices");
                egui::ComboBox::from_id_salt("angle-window")
                    .selected_text(format!("10–{}°", self.parameters.angle_max_deg))
                    .show_ui(ui, |ui| {
                        for m in [60., 90., 150.] {
                            ui.selectable_value(
                                &mut self.parameters.angle_max_deg,
                                m,
                                format!("10–{m}°"),
                            );
                        }
                    });
            });
            let rows = match self.rows(catalog) {
                Ok(r) => r,
                Err(e) => {
                    ui.colored_label(Color32::DARK_RED, e);
                    return;
                }
            };
            if !rows
                .iter()
                .any(|r| Some(&r.id) == self.selected_xrd.as_ref())
            {
                self.selected_xrd = rows.first().map(|r| r.id.clone());
            }
            let lanes: Vec<_> = catalog
                .phases
                .iter()
                .filter(|p| self.parameters.phases.contains(&p.id))
                .map(|p| p.label.clone())
                .collect();
            let markers: Vec<_> = rows
                .iter()
                .filter_map(|r| {
                    Some(Marker {
                        id: r.id.clone(),
                        lane: r.phase_label.clone(),
                        value: r.two_theta_deg?,
                        title: format!(
                            "{} ({})\nd={:.7} Å · 2θ={:.5}°",
                            r.phase_label,
                            hkl(r.displayed_hkl),
                            r.d_a,
                            r.two_theta_deg?
                        ),
                        enabled: true,
                    })
                })
                .collect();
            if let Some(id) = peak_plot::plot(
                ui,
                &markers,
                &lanes,
                self.parameters.angle_min_deg,
                self.parameters.angle_max_deg,
                "Scattering angle 2θ / degrees",
                self.selected_xrd.as_deref(),
                TEAL,
            ) {
                self.selected_xrd = Some(id);
            }
            ui.label(RichText::new("Selected symmetry-allowed positions; not a complete pattern or intensity prediction.").size(11.).color(MUTED));
            ui.label(format!(
                "{} reflections · {} outside plot / no solution",
                rows.len(),
                rows.iter()
                    .filter(|r| r
                        .two_theta_deg
                        .is_none_or(|a| a < self.parameters.angle_min_deg
                            || a > self.parameters.angle_max_deg))
                    .count()
            ));
            if let Some(r) = rows
                .iter()
                .find(|r| Some(&r.id) == self.selected_xrd.as_ref())
            {
                ui.separator();
                ui.heading(format!("{} ({})", r.phase_label, hkl(r.displayed_hkl)));
                ui.label(
                    RichText::new(r.two_theta_deg.map_or_else(
                        || "No first-order Bragg solution".into(),
                        |a| format!("{a:.5}° 2θ"),
                    ))
                    .size(22.)
                    .color(TEAL),
                );
                ui.label(format!("d = {:.7} Å · {}", r.d_a, r.setting));
                if let Some(p) = catalog.phases.iter().find(|p| p.id == r.phase_id) {
                    ui.hyperlink_to(format!("COD {} · source structure", p.cod), &p.source);
                }
                ui.label("Calculated from the reference cell and incident wavelength; this is not a measured peak.");
                if r.two_theta_deg.is_some_and(|a| {
                    a < self.parameters.angle_min_deg || a > self.parameters.angle_max_deg
                }) {
                    ui.colored_label(TEAL, "Selected reflection is outside the plot window.");
                }
            } else {
                ui.label("Select a crystalline phase to inspect reflections.");
            }
            let mut table_rows: Vec<_> = rows.iter().collect();
            if !full {
                table_rows.retain(|r| {
                    Some(&r.id) == self.selected_xrd.as_ref()
                        || r.two_theta_deg.is_some_and(|a| {
                            a >= self.parameters.angle_min_deg && a <= self.parameters.angle_max_deg
                        })
                });
                table_rows.sort_by_key(|r| Some(&r.id) != self.selected_xrd.as_ref());
            }
            egui::ScrollArea::both()
                .id_salt("xrd-table")
                .max_height(if full { 280. } else { 125. })
                .min_scrolled_height(if full { 240. } else { 125. })
                .show(ui, |ui| {
                    egui::Grid::new("xrd-grid")
                        .striped(true)
                        .spacing([16., 6.])
                        .show(ui, |ui| {
                            for label in ["Phase / h k l", "d (Å)", "2θ (degrees)"] {
                                ui.strong(label);
                            }
                            ui.end_row();
                            for r in table_rows.iter().take(if full { usize::MAX } else { 5 }) {
                                if ui
                                    .selectable_label(
                                        Some(&r.id) == self.selected_xrd.as_ref(),
                                        format!("{} ({})", r.phase_label, hkl(r.displayed_hkl)),
                                    )
                                    .clicked()
                                {
                                    self.selected_xrd = Some(r.id.clone());
                                }
                                ui.label(format!("{:.7}", r.d_a));
                                ui.label(
                                    r.two_theta_deg.map_or_else(
                                        || "No solution".into(),
                                        |a| format!("{a:.5}"),
                                    ),
                                );
                                ui.end_row();
                            }
                        });
                });
            if self.parameters.pnma {
                ui.label(
                    "Sb₂Se₃: (a',b',c')=(b,c,a); (h',k',l')=(k,l,h). d and 2θ stay unchanged.",
                );
            }
            if !full && ui.link("Inspect all selected XRD references ->").clicked() {
                self.view = View::Xrd;
            }
        });
    }
    fn snapshot(&self, catalog: &Catalog) -> Result<ReferenceSnapshot, String> {
        let mut p = self.parameters.clone();
        p.wavelength_a = self.wavelength(catalog)?;
        p.radiation_label = self.radiation_label(catalog);
        reference::snapshot(catalog, p, &self.interpretation).map_err(|e| e.to_string())
    }
    fn save_export(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &Catalog,
        store: Option<&Store>,
        experiments: &[CvtExperiment],
    ) {
        ui.add_space(16.);
        ui.separator();
        ui.collapsing("Save / export reference snapshot",|ui| {
            ui.label("Retains selected references, units, wavelength, beam, crystal settings, source hashes and interpretation.");
            ui.label("Researcher interpretation (optional)");ui.add(egui::TextEdit::multiline(&mut self.interpretation).desired_rows(3).desired_width(f32::INFINITY));
            let chosen=experiments.iter().find(|e|Some(&e.id)==self.target.as_ref());
            egui::ComboBox::from_id_salt("reference-log-target").selected_text(chosen.map_or("Choose an experiment",|e|e.name.as_str())).show_ui(ui,|ui| {
                for experiment in experiments{ui.selectable_value(&mut self.target,Some(experiment.id.clone()),&experiment.name);}
            });
            ui.horizontal_wrapped(|ui|{ui.label("Analysis event date");ui.add(egui::TextEdit::singleline(&mut self.event_date).desired_width(112.).hint_text("YYYY-MM-DD"));});
            let snapshot=self.snapshot(catalog);
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(snapshot.is_ok()&&chosen.is_some(),egui::Button::new("Save analysis to log")).clicked() {
                    let result=(|| {
                        let s=snapshot.as_ref().map_err(Clone::clone)?;
                        let date=chrono::NaiveDate::parse_from_str(self.event_date.trim(),"%Y-%m-%d").ok().filter(|d|d.to_string()==self.event_date.trim()).ok_or("Enter a valid event date in YYYY-MM-DD format.")?;
                        let target=chosen.ok_or("Choose an experiment.")?;
                        store.ok_or("Workspace unavailable.")?.create_entry(&EntryDraft{project_id:target.log_project_id.clone(),title:format!("Peak Explorer reference · {}",target.name),body:s.report().map_err(|e|e.to_string())?,event_date:date}).map_err(|e|e.to_string())?;
                        Ok::<_,String>(format!("Saved a new reference snapshot in {}.",target.name))
                    })();self.notice=Some(result.unwrap_or_else(|e|format!("Could not save: {e}")));
                }
                if ui.add_enabled(snapshot.is_ok(),egui::Button::new("Copy JSON")).clicked() {
                    match snapshot.as_ref().map_err(Clone::clone).and_then(|s|s.json().map_err(|e|e.to_string())) {
                        Ok(json)=>{ui.ctx().copy_text(json);self.notice=Some("Reference snapshot copied as JSON.".into());},Err(e)=>self.notice=Some(e),
                    }
                }
                if ui.add_enabled(snapshot.is_ok()&&store.is_some(),egui::Button::new("Export JSON to profile")).clicked() {
                    let result=snapshot.as_ref().map_err(Clone::clone).and_then(|s|{
                        let dir=store.and_then(|s|s.path().parent()).ok_or("Profile folder unavailable.")?;
                        reference::export_to_profile(s,dir).map_err(|e|e.to_string())
                    });self.notice=Some(result.map(|p|format!("Exported to {}",p.display())).unwrap_or_else(|e|format!("Could not export: {e}")));
                }
            });
            if let Err(e)=&snapshot{ui.colored_label(Color32::DARK_RED,e);}
            ui.label(RichText::new("Exports stay in this profile’s exports folder. Tool drafts remain in memory; save a snapshot to retain them.").size(11.).color(MUTED));
        });
        if let Some(n) = &self.notice {
            ui.label(RichText::new(n).color(TEAL));
        }
    }
}
fn toggle(values: &mut Vec<String>, value: &str, selected: bool) {
    values.retain(|v| v != value);
    if selected {
        values.push(value.into());
    }
}
fn hkl(v: [i32; 3]) -> String {
    format!("{} {} {}", v[0], v[1], v[2])
}
fn card(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(PAPER)
        .stroke(egui::Stroke::new(1., Color32::from_gray(220)))
        .corner_radius(8.)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
}
fn comparison(ui: &mut egui::Ui, c: &Catalog) {
    ui.add_space(16.);
    card(ui, |ui| {
        ui.heading("Same elements. Different signals.");
        ui.label("An Sb + Se mixture and Sb₂Se₃ can produce the same elemental EDX line families. EDX alone does not establish the Sb₂Se₃ phase.");
        if let Some(p) = c.phases.iter().find(|p| p.id == "sb2se3")
            && let Some(r) = p.reflections.iter().find(|r| r.hkl == [2, 4, 0])
        {
            let angles: Vec<_> = c
                .radiation
                .iter()
                .filter_map(|s| {
                    crate::peak::two_theta_deg(s.wavelength_a, r.d_a)
                        .ok()
                        .flatten()
                        .map(|a| format!("{}: {a:.3}°", s.label))
                })
                .collect();
            ui.colored_label(
                TEAL,
                format!(
                    "Change the XRD source -> Sb₂Se₃ (240) Pbnm / (402) Pnma shifts: {}.",
                    angles.join("; ")
                ),
            );
            ui.colored_label(PURPLE,"Sb Lα₁ remains 3.604756 keV. Changing the XRD source does not turn this emission line into a reflection.");
        }
        ui.label("E = hc/λ converts a photon’s energy to its wavelength. A Bragg angle additionally requires a crystal d-spacing: λ = 2d sin θ.");
    });
}
fn guide(ui: &mut egui::Ui, c: &Catalog) {
    ui.heading("Bench-side guide");
    egui::ScrollArea::horizontal()
        .id_salt("guide-table")
        .show(ui, |ui| {
            egui::Grid::new("guide-grid")
                .striped(true)
                .spacing([18., 10.])
                .show(ui, |ui| {
                    for t in ["Question", "EDX / EDS", "Angle-dispersive XRD"] {
                        ui.strong(t);
                    }
                    ui.end_row();
                    for row in [
                        [
                            "Signal",
                            "Electronic transition / emitted X-ray",
                            "Coherent scattering / crystal planes",
                        ],
                        [
                            "Horizontal axis",
                            "Photon energy E (keV)",
                            "Scattering angle 2θ (degrees)",
                        ],
                        [
                            "Peak label",
                            "Element + transition",
                            "Phase + Miller indices / setting",
                        ],
                        [
                            "Inference",
                            "Elemental presence; corrected composition",
                            "Crystal structure from multiple reflections",
                        ],
                        [
                            "Position depends on",
                            "Atomic electronic levels",
                            "Cell metric and incident wavelength",
                        ],
                        [
                            "Lab checks",
                            "Families, overlaps, beam, calibration",
                            "Wavelength, settings, texture, full pattern",
                        ],
                    ] {
                        for t in row {
                            ui.label(t);
                        }
                        ui.end_row();
                    }
                });
        });
    comparison(ui, c);
    ui.add_space(12.);
    ui.label("A reference transition may be blended or unresolved by EDX. Database uncertainties do not describe detector resolution. Being above an edge is only a necessary condition: detector response, matrix effects and overvoltage affect useful counts.");
    ui.label("A strong XRD reflection alone does not establish a growth direction or phase fraction. Check instrument calibration, sample displacement, strain, preferred orientation and the correct polymorph/temperature.");
    ui.label("An iodine EDX line does not distinguish residual transport agent, SbI₃ or SbSeI. Iodine-containing XRD phase references are not bundled in this catalog; do not substitute elemental emission lines for those diffraction patterns.");
    ui.separator();
    ui.heading("Reference data and provenance");
    ui.label(format!(
        "Data version {} · checked {} · {} EDX markers · {} CIFs · {} selected reflections",
        c.version,
        c.checked,
        c.edx.len(),
        c.phases.len(),
        c.phases.iter().map(|p| p.reflections.len()).sum::<usize>()
    ));
    ui.strong("NIST Standard Reference Database 128");
    ui.label("Deslattes et al., X-Ray Transition Energies Database. DOI: 10.18434/T4859Z. Uses the Direct experimental column, reported uncertainties and blend labels. Missing measured lines are excluded.");
    ui.horizontal_wrapped(|ui| {
        for e in &c.elements {
            ui.hyperlink_to(format!("{} table", e.symbol), &e.source);
        }
    });
    for p in &c.phases {
        ui.add_space(12.);
        ui.strong(format!("{} · COD {} · {}", p.label, p.cod, p.space_group));
        ui.label(&p.citation);
        ui.hyperlink_to("COD source structure", &p.source);
        ui.hyperlink_to(
            format!("DOI {}", p.doi),
            format!("https://doi.org/{}", p.doi),
        );
        ui.label(format!(
            "Source cell: a={}, b={}, c={} Å · α={}, β={}, γ={}°",
            p.cell_reported[0],
            p.cell_reported[1],
            p.cell_reported[2],
            p.angles_deg[0],
            p.angles_deg[1],
            p.angles_deg[2]
        ));
        ui.collapsing(format!("Source CIF and SHA-256 · {}", p.label), |ui| {
            ui.label(&p.sha256);
            egui::ScrollArea::both()
                .id_salt(("cif", &p.id))
                .max_height(180.)
                .min_scrolled_height(180.)
                .show(ui, |ui| {
                    ui.monospace(&p.cif_text);
                });
            if ui.button("Copy source CIF").clicked() {
                ui.ctx().copy_text(p.cif_text.clone());
            }
        });
    }
    ui.add_space(12.);
    ui.strong("Incident radiation");
    for r in &c.radiation {
        ui.label(format!(
            "{}: {} ± {} eV -> λ={:.10} Å (vacuum)",
            r.label, r.reported_energy_ev, r.reported_uncertainty_ev, r.wavelength_a
        ));
        ui.hyperlink_to(format!("NIST {} source", r.label), &r.source);
    }
    ui.label("Wavelength uses hc = 12398.419843320026 eV Å from exact SI definitions. Instrument conventions and unresolved Kα doublets can differ: use your instrument’s configured wavelength for matching.");
    ui.hyperlink_to(
        "NIST SI constants",
        "https://physics.nist.gov/cuu/Constants/",
    );
    ui.hyperlink_to(
        "IUCr: Bragg’s law",
        "https://dictionary.iucr.org/Bragg%27s_law",
    );
    ui.hyperlink_to(
        "EDX interpretation: Newbury & Ritchie (2019)",
        "https://doi.org/10.1017/S143192761901482X",
    );
    ui.collapsing("Source retrieval records and hashes", |ui| {
        for s in &c.sources {
            ui.label(format!("{} · {}", s.file, s.retrieved_utc));
            ui.label(&s.sha256);
            ui.hyperlink_to("Retrieved source", &s.url);
        }
    });
}
