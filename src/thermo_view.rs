use eframe::egui::{self, Color32, RichText};

use crate::{
    app::{INK, MUTED, TEAL, primary_button},
    thermo_database::{Phase, ThermoEntry, ThermoEntryDraft, ThermoStore},
    thermodynamics::HeatModel,
};

#[derive(Default)]
struct SourceForm {
    name: String,
    authors: String,
    year: String,
    reference: String,
}

struct EntryForm {
    id: Option<String>,
    revision: i64,
    source_id: String,
    species: String,
    formula: String,
    phase: Phase,
    min_k: String,
    max_k: String,
    reference_k: String,
    reference_pressure_bar: String,
    enthalpy: String,
    entropy: String,
    heat_capacity: String,
    model_kind: u8,
    model_a: String,
    model_b: String,
    model_c: String,
    model_d: String,
    locator: String,
    comments: String,
}

impl EntryForm {
    fn new(source_id: String) -> Self {
        Self {
            id: None,
            revision: 0,
            source_id,
            species: String::new(),
            formula: String::new(),
            phase: Phase::Solid,
            min_k: String::new(),
            max_k: String::new(),
            reference_k: "298.15".into(),
            reference_pressure_bar: String::new(),
            enthalpy: String::new(),
            entropy: String::new(),
            heat_capacity: String::new(),
            model_kind: 0,
            model_a: String::new(),
            model_b: String::new(),
            model_c: String::new(),
            model_d: String::new(),
            locator: String::new(),
            comments: String::new(),
        }
    }
    fn from_entry(entry: &ThermoEntry) -> Self {
        let draft = &entry.draft;
        Self {
            id: Some(entry.id.clone()),
            revision: entry.revision,
            source_id: draft.source_id.clone(),
            species: draft.species.clone(),
            formula: draft.formula.clone(),
            phase: draft.phase,
            min_k: draft.min_k.to_string(),
            max_k: draft.max_k.to_string(),
            reference_k: draft.reference_k.to_string(),
            reference_pressure_bar: draft.reference_pressure_bar.to_string(),
            enthalpy: draft
                .enthalpy_j_mol
                .map_or(String::new(), |value| value.to_string()),
            entropy: draft
                .entropy_j_mol_k
                .map_or(String::new(), |value| value.to_string()),
            heat_capacity: draft
                .heat_capacity_j_mol_k
                .map_or(String::new(), |value| value.to_string()),
            model_kind: match draft.model {
                HeatModel::ReferenceOnly => 0,
                HeatModel::ConstantCp { .. } => 1,
                HeatModel::PolynomialCp { .. } => 2,
            },
            model_a: match draft.model {
                HeatModel::ConstantCp {
                    joule_per_mol_kelvin,
                } => joule_per_mol_kelvin.to_string(),
                HeatModel::PolynomialCp { a, .. } => a.to_string(),
                _ => String::new(),
            },
            model_b: match draft.model {
                HeatModel::PolynomialCp { b, .. } => b.to_string(),
                _ => String::new(),
            },
            model_c: match draft.model {
                HeatModel::PolynomialCp { c, .. } => c.to_string(),
                _ => String::new(),
            },
            model_d: match draft.model {
                HeatModel::PolynomialCp { d, .. } => d.to_string(),
                _ => String::new(),
            },
            locator: draft.source_locator.clone(),
            comments: draft.comments.clone(),
        }
    }
    fn draft(&self) -> Result<ThermoEntryDraft, String> {
        fn required(value: &str, label: &str) -> Result<f64, String> {
            value
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("Enter {label} as a number."))
        }
        fn optional(value: &str, label: &str) -> Result<Option<f64>, String> {
            if value.trim().is_empty() {
                Ok(None)
            } else {
                required(value, label).map(Some)
            }
        }
        Ok(ThermoEntryDraft {
            source_id: self.source_id.clone(),
            species: self.species.clone(),
            formula: self.formula.clone(),
            phase: self.phase,
            min_k: required(&self.min_k, "minimum temperature")?,
            max_k: required(&self.max_k, "maximum temperature")?,
            reference_k: required(&self.reference_k, "reference temperature")?,
            reference_pressure_bar: required(&self.reference_pressure_bar, "standard pressure")?,
            enthalpy_j_mol: optional(&self.enthalpy, "enthalpy")?,
            entropy_j_mol_k: optional(&self.entropy, "entropy")?,
            heat_capacity_j_mol_k: optional(&self.heat_capacity, "heat capacity")?,
            model: match self.model_kind {
                0 => HeatModel::ReferenceOnly,
                1 => HeatModel::ConstantCp {
                    joule_per_mol_kelvin: required(&self.model_a, "constant Cp")?,
                },
                _ => HeatModel::PolynomialCp {
                    a: required(&self.model_a, "Cp a")?,
                    b: required(&self.model_b, "Cp b")?,
                    c: required(&self.model_c, "Cp c")?,
                    d: required(&self.model_d, "Cp d")?,
                },
            },
            source_locator: self.locator.clone(),
            comments: self.comments.clone(),
        })
    }
}

enum ViewAction {
    AddSource,
    ImportWorksheetCandidates,
    SetPrimary(String),
    ToggleSource(String, bool),
    DeleteSource(String),
    SaveEntry,
    Verify(String, i64, bool),
}

pub struct ThermoDatabaseView {
    selected_source_id: String,
    source_form: Option<SourceForm>,
    entry_form: Option<EntryForm>,
    notice: String,
    error: Option<String>,
    verification_pending: Option<(String, i64)>,
}

impl Default for ThermoDatabaseView {
    fn default() -> Self {
        Self {
            selected_source_id: "knacke-1991".into(),
            source_form: None,
            entry_form: None,
            notice: String::new(),
            error: None,
            verification_pending: None,
        }
    }
}

impl ThermoDatabaseView {
    pub fn has_unsaved_form(&self) -> bool {
        self.source_form.is_some() || self.entry_form.is_some()
    }
    pub fn last_error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn discard_form(&mut self) {
        self.source_form = None;
        self.entry_form = None;
        self.error = None;
        self.verification_pending = None;
    }
    pub fn save_form(&mut self, store: Option<&ThermoStore>) -> bool {
        let Some(store) = store else {
            self.error = Some("The local thermodynamic database is unavailable.".into());
            return false;
        };
        if self.source_form.is_some() {
            self.apply(store, ViewAction::AddSource);
        } else if self.entry_form.is_some() {
            self.apply(store, ViewAction::SaveEntry);
        }
        self.error.is_none()
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        store: Option<&ThermoStore>,
        startup_error: Option<&str>,
    ) {
        let Some(store) = store else {
            ui.heading("Thermodynamic Databases");
            ui.colored_label(
                Color32::DARK_RED,
                startup_error.unwrap_or("The local thermodynamic database is unavailable."),
            );
            return;
        };
        let sources = match store.sources() {
            Ok(sources) => sources,
            Err(error) => {
                ui.colored_label(Color32::DARK_RED, error.to_string());
                return;
            }
        };
        if !sources
            .iter()
            .any(|source| source.id == self.selected_source_id)
        {
            self.selected_source_id = store
                .primary_source_id()
                .unwrap_or_else(|_| "knacke-1991".into());
        }
        let primary = store.primary_source_id().ok();
        let selected = sources
            .iter()
            .find(|source| source.id == self.selected_source_id);
        let entries = match store.entries(&self.selected_source_id) {
            Ok(entries) => entries,
            Err(error) => {
                ui.colored_label(Color32::DARK_RED, error.to_string());
                return;
            }
        };
        let mut action = None;
        egui::ScrollArea::vertical().id_salt("thermo-databases").show(ui,|ui| {
            egui::Frame::new().inner_margin(26).show(ui,|ui| {
                ui.heading(RichText::new("Thermodynamic Databases").size(27.0).color(INK));
                ui.label(RichText::new("Local reference records · no thermodynamic values are bundled").color(MUTED));
                ui.add_space(14.0);
                if let Some(error)=&self.error {ui.colored_label(Color32::DARK_RED,error);}
                if !self.notice.is_empty() {ui.colored_label(TEAL,&self.notice);}
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Database");
                    egui::ComboBox::from_id_salt("thermo-source").selected_text(selected.map_or("Choose database",|source|source.name.as_str())).show_ui(ui,|ui| {
                        for source in &sources {
                            if ui.selectable_label(source.id==self.selected_source_id,&source.name).clicked() {
                                if self.has_unsaved_form() {self.error=Some("Save or cancel the open form before changing databases.".into());}
                                else {self.selected_source_id=source.id.clone();self.error=None;}
                            }
                        }
                    });
                    if ui.button("+ Add database").clicked() {
                        if self.has_unsaved_form() {self.error=Some("Save or cancel the open form first.".into());}
                        else {self.source_form=Some(SourceForm::default());self.error=None;}
                    }
                    if ui.button("Import supplied worksheet candidates").clicked() {
                        action=Some(ViewAction::ImportWorksheetCandidates);
                    }
                });
                if let Some(source)=selected {
                    ui.label(format!("{} · {} · {}",source.authors,source.year,source.reference));
                    ui.horizontal(|ui| {
                        if Some(&source.id)==primary.as_ref() {ui.colored_label(TEAL,"Primary database");}
                        else if source.enabled && ui.button("Set as primary").clicked() {action=Some(ViewAction::SetPrimary(source.id.clone()));}
                        ui.label(if source.enabled {"Enabled"} else {"Disabled"});
                        if ui.button(if source.enabled {"Disable"} else {"Enable"}).clicked() {action=Some(ViewAction::ToggleSource(source.id.clone(),!source.enabled));}
                        if source.id!="knacke-1991" && ui.button("Delete empty database").clicked() {action=Some(ViewAction::DeleteSource(source.id.clone()));}
                    });
                }
                ui.label(RichText::new("Fallback database: None. Values from different sources are never combined automatically.").size(12.0).color(MUTED));
                if let Some(form)=&mut self.source_form {
                    ui.separator(); ui.strong("Add source database");
                    field(ui,"Database name",&mut form.name,"e.g. Custom Laboratory Database");
                    field(ui,"Authors or organisation",&mut form.authors,"");
                    field(ui,"Year",&mut form.year,"YYYY");
                    field(ui,"Reference",&mut form.reference,"Publication or internal reference");
                    ui.horizontal(|ui| {
                        if primary_button(ui,"Create database").clicked() {action=Some(ViewAction::AddSource);}
                        if ui.button("Cancel").clicked(){self.source_form=None;}
                    });
                }
                ui.add_space(18.0); ui.separator(); ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.heading("Species records");
                    if ui.button("+ Manual entry").clicked() {
                        if self.has_unsaved_form(){self.error=Some("Save or cancel the open form first.".into());}
                        else {self.entry_form=Some(EntryForm::new(self.selected_source_id.clone()));self.error=None;}
                    }
                });
                ui.label(RichText::new("One phase and validated temperature range per record. New and edited records are unverified until you review the cited source.").size(12.0).color(MUTED));
                if entries.is_empty() {ui.label("No species data entered for this database.");}
                for entry in &entries {
                    ui.push_id(&entry.id,|ui| {
                        egui::Frame::group(ui.style()).show(ui,|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.strong(format!("{} ({})",entry.draft.formula,entry.draft.phase.code()));
                                ui.label(format!("{}–{} K",entry.draft.min_k,entry.draft.max_k));
                                ui.colored_label(if entry.verified_at.is_some(){TEAL}else{MUTED},if entry.verified_at.is_some(){"Verified"}else{"Unverified"});
                                if ui.button("Inspect / edit").clicked(){
                                    if self.has_unsaved_form(){self.error=Some("Save or cancel the open form first.".into());}
                                    else {self.entry_form=Some(EntryForm::from_entry(entry));self.error=None;}
                                }
                                if ui.button(if entry.verified_at.is_some(){"Mark unverified"}else{"Mark verified"}).clicked(){action=Some(ViewAction::Verify(entry.id.clone(),entry.revision,entry.verified_at.is_none()));}
                            });
                            ui.label(format!("{} · source: {}",entry.draft.species,entry.draft.source_locator));
                            if entry.draft.comments.contains("DISCREPANCY") {
                                ui.colored_label(Color32::DARK_RED,&entry.draft.comments);
                            }
                            if let Some(date)=entry.verified_at {ui.label(RichText::new(format!("Verified {} UTC",date.format("%Y-%m-%d %H:%M"))).size(11.0).color(MUTED));}
                        });
                    });
                }
                if let Some(form)=&mut self.entry_form {
                    ui.add_space(14.0);ui.separator();
                    ui.heading(if form.id.is_some(){"Edit species record"}else{"Manual species entry"});
                    ui.colored_label(Color32::from_rgb(150,90,35),"Compare every value, unit, phase and range with the named source before marking verified. Editing clears verification.");
                    field(ui,"Species name",&mut form.species,"e.g. antimony trichloride");
                    field(ui,"Chemical formula",&mut form.formula,"e.g. SbCl3");
                    egui::ComboBox::from_id_salt("thermo-phase").selected_text(form.phase.label()).show_ui(ui,|ui| {
                        for phase in Phase::ALL {ui.selectable_value(&mut form.phase,phase,phase.label());}
                    });
                    ui.columns(3,|columns| {
                        field(&mut columns[0],"Minimum K",&mut form.min_k,"K");
                        field(&mut columns[1],"Maximum K",&mut form.max_k,"K");
                        field(&mut columns[2],"Reference K",&mut form.reference_k,"298.15");
                    });
                    ui.horizontal(|ui| {
                        ui.strong("Temperature model");
                        egui::ComboBox::from_id_salt("thermo-model").selected_text(match form.model_kind {0=>"Reference point only",1=>"Constant Cp",_=>"Cp = a + bT + c/T² + dT²"}).show_ui(ui,|ui| {
                            ui.selectable_value(&mut form.model_kind,0,"Reference point only");
                            ui.selectable_value(&mut form.model_kind,1,"Constant Cp");
                            ui.selectable_value(&mut form.model_kind,2,"Cp = a + bT + c/T² + dT²");
                        });
                    });
                    if form.model_kind != 0 {
                        field(ui,if form.model_kind==1{"Cp · J/(mol·K)"}else{"a · J/(mol·K)"},&mut form.model_a,"required");
                    }
                    if form.model_kind == 2 {
                        ui.columns(3,|columns| {
                            field(&mut columns[0],"b · J/(mol·K²)",&mut form.model_b,"required");
                            field(&mut columns[1],"c · J·K/mol",&mut form.model_c,"required");
                            field(&mut columns[2],"d · J/(mol·K³)",&mut form.model_d,"required");
                        });
                    }
                    field(ui,"Standard pressure · bar",&mut form.reference_pressure_bar,"Required · check the source convention");
                    ui.columns(3,|columns| {
                        field(&mut columns[0],"ΔfH° at reference · J/mol",&mut form.enthalpy,"optional");
                        field(&mut columns[1],"S° at reference · J/(mol·K)",&mut form.entropy,"optional");
                        field(&mut columns[2],"Cp° at reference · J/(mol·K)",&mut form.heat_capacity,"optional");
                    });
                    field(ui,"Source page / table",&mut form.locator,"Required · page and table identifier");
                    ui.label("Comments");ui.add(egui::TextEdit::multiline(&mut form.comments).desired_rows(3).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        if primary_button(ui,"Save unverified record").clicked(){action=Some(ViewAction::SaveEntry);}
                        if ui.button("Cancel").clicked(){self.entry_form=None;self.error=None;}
                    });
                }
            });
        });
        if let Some(action) = action {
            self.apply(store, action);
        }
        if let Some((id, revision)) = self.verification_pending.clone() {
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Verify thermodynamic entry")
                .collapsible(false).resizable(false).default_width(520.0).show(ui.ctx(),|ui| {
                    match store.entry(&id) {
                        Ok(entry) => {
                            ui.strong(format!("{} ({})",entry.draft.formula,entry.draft.phase.code()));
                            ui.label(format!("Database: {}",sources.iter().find(|source|source.id==entry.draft.source_id).map_or(entry.draft.source_id.as_str(),|source|source.name.as_str())));
                            ui.label(format!("Source locator: {}",entry.draft.source_locator));
                            ui.label(format!("Validity: {}–{} K; reference: {} K, {} bar",entry.draft.min_k,entry.draft.max_k,entry.draft.reference_k,entry.draft.reference_pressure_bar));
                            for (label,value,unit) in [("ΔfH°",entry.draft.enthalpy_j_mol,"J/mol"),("S°",entry.draft.entropy_j_mol_k,"J/(mol·K)"),("Cp°",entry.draft.heat_capacity_j_mol_k,"J/(mol·K)")] {
                                if let Some(value)=value {ui.label(format!("{label}: {value} {unit}"));}
                            }
                            match entry.draft.model {
                                HeatModel::ReferenceOnly=>{ui.label("Temperature model: reference point only");}
                                HeatModel::ConstantCp{joule_per_mol_kelvin}=>{ui.label(format!("Constant Cp: {joule_per_mol_kelvin} J/(mol·K)"));}
                                HeatModel::PolynomialCp{a,b,c,d}=>{ui.label(format!("Cp(T) = a + bT + c/T² + dT²; a={a}, b={b}, c={c}, d={d} (T in K, Cp in J/(mol·K))"));}
                            }
                            if !entry.draft.comments.trim().is_empty(){ui.colored_label(Color32::from_rgb(150,90,35),&entry.draft.comments);}
                            ui.colored_label(Color32::from_rgb(150,90,35),"Mark verified only after checking every value, unit, phase and range against the named source. Worksheet records do not establish verification against Knacke.");
                            ui.horizontal(|ui| {
                                if primary_button(ui,"I checked this source").clicked(){confirm=true;}
                                if ui.button("Cancel").clicked(){cancel=true;}
                            });
                        }
                        Err(error) => {ui.colored_label(Color32::DARK_RED,error.to_string());if ui.button("Close").clicked(){cancel=true;}}
                    }
                });
            if cancel {
                self.verification_pending = None;
            }
            if confirm {
                self.verification_pending = None;
                match store.set_verified(&id, revision, true) {
                    Ok(_) => {
                        self.notice = "Record marked verified".into();
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
        }
    }

    fn apply(&mut self, store: &ThermoStore, action: ViewAction) {
        self.notice.clear();
        self.error = None;
        let result: Result<String, String> = match action {
            ViewAction::ImportWorksheetCandidates => store.import_worksheet_candidates(include_str!("../data/thermo/worksheet_candidates.json"))
                .map(|count|format!("Imported {count} unverified worksheet candidates. Select each worksheet source to review them."))
                .map_err(|error|error.to_string()),
            ViewAction::AddSource => {
                if let Some(form) = &self.source_form {
                    match form.year.trim().parse::<i32>() {
                        Ok(year) => store
                            .create_source(&form.name, &form.authors, year, &form.reference)
                            .map(|source| {
                                self.selected_source_id = source.id;
                                self.source_form = None;
                                "Database created".into()
                            })
                            .map_err(|error| error.to_string()),
                        Err(_) => Err("Enter a valid publication year.".into()),
                    }
                } else {
                    Ok(String::new())
                }
            }
            ViewAction::SetPrimary(id) => store
                .set_primary_source(&id)
                .map(|()| "Primary database updated".into())
                .map_err(|error| error.to_string()),
            ViewAction::ToggleSource(id, enabled) => store
                .set_source_enabled(&id, enabled)
                .map(|()| "Database availability updated".into())
                .map_err(|error| error.to_string()),
            ViewAction::DeleteSource(id) => store
                .delete_empty_source(&id)
                .map(|()| {
                    self.selected_source_id = store
                        .primary_source_id()
                        .unwrap_or_else(|_| "knacke-1991".into());
                    "Empty database deleted".into()
                })
                .map_err(|error| error.to_string()),
            ViewAction::SaveEntry => {
                if let Some(form) = &self.entry_form {
                    form.draft()
                        .and_then(|draft| {
                            if let Some(id) = &form.id {
                                store.update_entry(id, form.revision, &draft)
                            } else {
                                store.create_entry(&draft)
                            }
                            .map_err(|error| error.to_string())
                        })
                        .map(|_| {
                            self.entry_form = None;
                            "Unverified record saved".into()
                        })
                } else {
                    Ok(String::new())
                }
            }
            ViewAction::Verify(id, revision, verified) => {
                if verified {
                    self.verification_pending = Some((id, revision));
                    Ok(String::new())
                } else {
                    store
                        .set_verified(&id, revision, false)
                        .map(|_| "Record marked unverified".into())
                        .map_err(|error| error.to_string())
                }
            }
        };
        match result {
            Ok(notice) => self.notice = notice,
            Err(error) => self.error = Some(error),
        }
    }
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(label);
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(f32::INFINITY),
    );
}
