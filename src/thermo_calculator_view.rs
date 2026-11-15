use std::{collections::BTreeMap, sync::Arc};

use eframe::egui::{self, Color32, RichText};
use serde_json::json;

use crate::{
    app::{INK, MUTED, TEAL, primary_button},
    store::Store,
    thermo_database::ThermoStore,
    thermodynamics::{
        PhaseMode, PhaseSeries, calculate_phase_series, equilibrium_gas_pressure, parse_reaction,
        standard_electrode_potential,
    },
    xlsx_template::{ExportRow, export_template},
};

pub struct ThermoCalculatorView {
    source_id: String,
    name: String,
    equation: String,
    start_k: String,
    end_k: String,
    step_k: String,
    mode: usize,
    fixed_phases: String,
    target_gas: String,
    other_gas_activities: String,
    electron_count: String,
    export_path: String,
    template_path: String,
    excel_path: String,
    result: Option<Arc<PhaseSeries>>,
    result_signature: String,
    error: Option<String>,
    notice: String,
    selected_snapshot: Option<String>,
}

impl Default for ThermoCalculatorView {
    fn default() -> Self {
        Self {
            source_id: String::new(),
            name: "Chlorine equilibrium".into(),
            equation: "2/3 SbCl3(g) -> 2/3 Sb + Cl2(g)".into(),
            start_k: "923".into(),
            end_k: "923".into(),
            step_k: "25".into(),
            mode: 0,
            fixed_phases: "Sb=l".into(),
            target_gas: "Cl2".into(),
            other_gas_activities: "SbCl3=1".into(),
            electron_count: String::new(),
            export_path: String::new(),
            template_path: String::new(),
            excel_path: String::new(),
            result: None,
            result_signature: String::new(),
            error: None,
            notice: String::new(),
            selected_snapshot: None,
        }
    }
}

impl ThermoCalculatorView {
    fn signature(&self) -> String {
        format!(
            "{:?}",
            (
                &self.source_id,
                &self.name,
                &self.equation,
                &self.start_k,
                &self.end_k,
                &self.step_k,
                self.mode,
                &self.fixed_phases,
                &self.target_gas,
                &self.other_gas_activities,
                &self.electron_count
            )
        )
    }
    fn calculate(&mut self, database: &ThermoStore) -> Result<(), String> {
        let reaction = parse_reaction(&self.name, &self.equation).map_err(|e| e.to_string())?;
        let parse_k = |value: &str| {
            value
                .trim()
                .parse::<f64>()
                .map_err(|_| "Enter numeric temperatures in K.".to_string())
        };
        let start = parse_k(&self.start_k)?;
        let end = parse_k(&self.end_k)?;
        let step = parse_k(&self.step_k)?;
        if !start.is_finite()
            || !end.is_finite()
            || !step.is_finite()
            || start <= 0.0
            || end < start
            || step <= 0.0
        {
            return Err("Enter positive temperatures and step, with end ≥ start.".into());
        }
        let count = ((end - start) / step).floor();
        if count > 999.0 {
            return Err("Limit a calculation to 1,000 points.".into());
        }
        let mut temperatures = (0..=count as usize)
            .map(|i| start + i as f64 * step)
            .collect::<Vec<_>>();
        if temperatures.last().is_some_and(|last| *last < end - 1e-8) {
            temperatures.push(end);
        }
        let mode = match self.mode {
            0 => PhaseMode::Automatic,
            1 => {
                let mut forced = BTreeMap::new();
                for pair in self
                    .fixed_phases
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let (formula, phase) = pair
                        .split_once('=')
                        .ok_or("Use formula=phase, e.g. Sb=l.")?;
                    forced.insert(formula.trim().into(), phase.trim().into());
                }
                PhaseMode::Fixed(forced)
            }
            _ => PhaseMode::Explicit,
        };
        let records = database
            .engine_records(&self.source_id)
            .map_err(|e| e.to_string())?;
        let series =
            calculate_phase_series(&reaction, &temperatures, &self.source_id, &records, &mode)
                .map_err(|e| e.to_string())?;
        self.result = Some(Arc::new(series));
        self.result_signature = self.signature();
        self.notice = format!("Calculated {} temperature point(s).", temperatures.len());
        Ok(())
    }

    fn gas_pressure(
        &self,
        point: &crate::thermodynamics::Calculation,
    ) -> Result<Option<(f64, f64)>, String> {
        if self.target_gas.trim().is_empty() {
            return Ok(None);
        }
        let target = point
            .species_used
            .iter()
            .find(|used| used.record.formula == self.target_gas.trim() && used.record.phase == "g")
            .ok_or_else(|| "The target gas is not present in this reaction.".to_string())?;
        let mut activities = BTreeMap::new();
        for pair in self
            .other_gas_activities
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let (formula, value) = pair
                .split_once('=')
                .ok_or("Use gas=activity, e.g. SbCl3=1.")?;
            let used = point
                .species_used
                .iter()
                .find(|used| used.record.formula == formula.trim() && used.record.phase == "g")
                .ok_or_else(|| format!("Gas {} is not in the reaction.", formula.trim()))?;
            activities.insert(
                used.record.id.clone(),
                value
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Enter numeric gas activities.".to_string())?,
            );
        }
        equilibrium_gas_pressure(point, &target.record.id, &activities)
            .map(Some)
            .map_err(|e| e.to_string())
    }

    fn snapshot(&self, series: &PhaseSeries) -> serde_json::Value {
        json!({
            "format":"CrystalForge thermodynamic calculation v1",
            "name":self.name,"equation":self.equation,"phase_mode":self.mode,
            "fixed_phases":self.fixed_phases,"source_id":self.source_id,
            "target_gas":self.target_gas,"other_gas_activities":self.other_gas_activities,
            "electron_count":self.electron_count,
            "phase_changes":series.changes.iter().map(|change|json!({
                "formula":change.formula,"from":change.from_phase,"to":change.to_phase,
                "from_record_id":change.from_record_id,"to_record_id":change.to_record_id,
                "between_k":change.between_k,
            })).collect::<Vec<_>>(),
            "points":series.points.iter().map(|point|json!({
                "temperature_k":point.temperature_k,"standard_pressure_bar":point.standard_pressure_bar,
                "delta_h_j_mol":point.delta_h_j_mol,"delta_s_j_mol_k":point.delta_s_j_mol_k,
                "delta_g_j_mol":point.delta_g_j_mol,"ln_k":point.ln_k,"log10_k":point.log10_k,
                "equilibrium_constant":point.equilibrium_constant,
                "gas_pressure_bar":self.gas_pressure(point).ok().flatten().map(|(_,bar)|bar),
                "log10_gas_pressure_over_standard":self.gas_pressure(point).ok().flatten().map(|(relative,_)|relative.log10()),
                "gas_pressure_error":self.gas_pressure(point).err(),
                "electrode_potential_v":self.electron_count.trim().parse::<u32>().ok()
                    .and_then(|z|standard_electrode_potential(point,z).ok()),
                "species":point.species_used.iter().map(|used|json!({
                    "coefficient_numerator":used.coefficient.numerator().to_string(),
                    "coefficient_denominator":used.coefficient.denominator().to_string(),
                    "id":used.record.id,"revision":used.record.revision,
                    "species_name":used.record.species_name,"comments":used.record.comments,
                    "imported_at":used.record.imported_at.to_rfc3339(),
                    "updated_at":used.record.updated_at.to_rfc3339(),
                    "formula":used.record.formula,"phase":used.record.phase,
                    "source_id":used.record.source_id,"source_name":used.record.source_name,
                    "source_enabled":used.record.source_enabled,
                    "source_authors":used.record.source_authors,"source_year":used.record.source_year,
                    "source_reference":used.record.source_reference,"source_locator":used.record.source_locator,
                    "verified_at":used.record.verified_at.map(|date|date.to_rfc3339()),
                    "min_k":used.record.min_k,"max_k":used.record.max_k,
                    "reference_k":used.record.reference_k,"reference_pressure_bar":used.record.reference_pressure_bar,
                    "formation_enthalpy_j_mol":used.record.formation_enthalpy_j_mol,
                    "molar_entropy_j_mol_k":used.record.molar_entropy_j_mol_k,
                    "model":format!("{:?}",used.record.model),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    }

    fn csv(&self, series: &PhaseSeries) -> Result<String, String> {
        let mut csv="temperature_K,delta_H_J_per_mol,delta_S_J_per_mol_K,delta_G_J_per_mol,ln_K,log10_K,K,gas_pressure_bar,log10_gas_pressure_over_standard,electrode_potential_V,species_provenance\n".to_string();
        let z = if self.electron_count.trim().is_empty() {
            None
        } else {
            Some(
                self.electron_count
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Enter a positive integer z.".to_string())?,
            )
        };
        for point in &series.points {
            let gas = self.gas_pressure(point)?;
            let potential = z
                .map(|z| standard_electrode_potential(point, z))
                .transpose()
                .map_err(|e| e.to_string())?;
            let provenance = point
                .species_used
                .iter()
                .map(|used| {
                    format!(
                        "{}({}) [{} rev {}: {}]",
                        used.record.formula,
                        used.record.phase,
                        used.record.source_name,
                        used.record.revision,
                        used.record.source_locator
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            let quote = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{}\n",
                point.temperature_k,
                point.delta_h_j_mol,
                point.delta_s_j_mol_k,
                point.delta_g_j_mol,
                point.ln_k,
                point.log10_k,
                point
                    .equilibrium_constant
                    .map_or(String::new(), |v| v.to_string()),
                gas.map_or(String::new(), |(_, p)| p.to_string()),
                gas.map_or(String::new(), |(relative, _)| relative.log10().to_string()),
                potential.map_or(String::new(), |v| v.to_string()),
                quote(&provenance)
            ));
        }
        Ok(csv)
    }

    fn excel_rows(&self, series: &PhaseSeries) -> Result<Vec<ExportRow>, String> {
        let z = if self.electron_count.trim().is_empty() {
            None
        } else {
            Some(
                self.electron_count
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Enter a positive integer z.".to_string())?,
            )
        };
        series
            .points
            .iter()
            .map(|point| {
                let gas = self.gas_pressure(point)?;
                let potential = z
                    .map(|count| {
                        standard_electrode_potential(point, count).map_err(|e| e.to_string())
                    })
                    .transpose()?;
                Ok(ExportRow {
                    temperature_k: point.temperature_k,
                    delta_h_j_mol: point.delta_h_j_mol,
                    delta_s_j_mol_k: point.delta_s_j_mol_k,
                    delta_g_j_mol: point.delta_g_j_mol,
                    log10_k: point.log10_k,
                    gas_pressure_bar: gas.map(|(_, bar)| bar),
                    log10_gas_pressure: gas.map(|(relative, _)| relative.log10()),
                    electrode_potential_v: potential,
                    phases: point
                        .species_used
                        .iter()
                        .map(|used| format!("{}({})", used.record.formula, used.record.phase))
                        .collect::<Vec<_>>()
                        .join("; "),
                    provenance: point
                        .species_used
                        .iter()
                        .map(|used| {
                            format!(
                                "{} rev {}: {}",
                                used.record.source_name,
                                used.record.revision,
                                used.record.source_locator
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                })
            })
            .collect()
    }

    fn load_saved(&mut self, snapshot: &str) -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(snapshot).map_err(|e| e.to_string())?;
        let string = |key: &str| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        self.name = string("name");
        self.equation = string("equation");
        self.source_id = string("source_id");
        self.fixed_phases = string("fixed_phases");
        self.target_gas = string("target_gas");
        self.other_gas_activities = string("other_gas_activities");
        self.electron_count = string("electron_count");
        self.mode = value
            .get("phase_mode")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .min(2) as usize;
        if let Some(points) = value.get("points").and_then(serde_json::Value::as_array) {
            if let Some(t) = points
                .first()
                .and_then(|p| p.get("temperature_k"))
                .and_then(serde_json::Value::as_f64)
            {
                self.start_k = t.to_string();
            }
            if let Some(t) = points
                .last()
                .and_then(|p| p.get("temperature_k"))
                .and_then(serde_json::Value::as_f64)
            {
                self.end_k = t.to_string();
            }
        }
        self.result = None;
        self.notice="Saved reaction loaded. Recalculate with current records; the old snapshot remains unchanged.".into();
        Ok(())
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        database: Option<&ThermoStore>,
        profile: Option<&Store>,
        workbook_id: Option<&str>,
    ) {
        let Some(database) = database else {
            ui.colored_label(
                Color32::DARK_RED,
                "Thermodynamic reference database unavailable.",
            );
            return;
        };
        let sources = match database.sources() {
            Ok(s) => s,
            Err(e) => {
                ui.colored_label(Color32::DARK_RED, e.to_string());
                return;
            }
        };
        if self.source_id.is_empty() {
            self.source_id = database.primary_source_id().unwrap_or_default();
        }
        egui::ScrollArea::vertical().id_salt("thermo-calculator").show(ui,|ui| {
            egui::Frame::new().inner_margin(26).show(ui,|ui| {
                ui.heading(RichText::new("Thermodynamic Calculator").size(27.0).color(INK));
                ui.label(RichText::new("Verified local records only · no extrapolation or silent source mixing").color(MUTED));
                ui.add_space(12.0);
                if let Some(error)=&self.error {ui.colored_label(Color32::DARK_RED,error);}
                if !self.notice.is_empty(){ui.colored_label(TEAL,&self.notice);}
                ui.horizontal(|ui| {
                    ui.strong("Database");
                    let selected=sources.iter().find(|source|source.id==self.source_id);
                    egui::ComboBox::from_id_salt("calculator-source").selected_text(selected.map_or("Select source",|source|source.name.as_str())).show_ui(ui,|ui|{
                        for source in &sources {if source.enabled && ui.selectable_value(&mut self.source_id,source.id.clone(),&source.name).changed(){self.result=None;}}
                    });
                    if let Ok(records)=database.engine_records(&self.source_id){ui.label(format!("{} verified / {} records",records.iter().filter(|r|r.verified_at.is_some()).count(),records.len()));}
                });
                ui.add_space(8.0);
                field(ui,"Calculation name",&mut self.name,"Name");
                field(ui,"Reaction · use -> and optional (s), (l), (g)",&mut self.equation,"2/3 SbCl3(g) -> 2/3 Sb + Cl2(g)");
                ui.horizontal(|ui| {
                    ui.strong("Phase mode");
                    egui::ComboBox::from_id_salt("calculator-mode").selected_text(match self.mode{0=>"Automatic",1=>"Fixed overrides",_=>"Explicit reaction"}).show_ui(ui,|ui|{
                        ui.selectable_value(&mut self.mode,0,"Automatic");
                        ui.selectable_value(&mut self.mode,1,"Fixed overrides");
                        ui.selectable_value(&mut self.mode,2,"Explicit reaction");
                    });
                });
                if self.mode==1 {field(ui,"Fixed phase overrides · formula=phase, comma separated",&mut self.fixed_phases,"Sb=l");}
                ui.label(RichText::new("A phase written in a term stays fixed. Automatic mode selects only phase-free terms from nonoverlapping valid ranges.").size(12.0).color(MUTED));
                ui.columns(3,|columns|{
                    field(&mut columns[0],"Start · K",&mut self.start_k,"923");
                    field(&mut columns[1],"End · K",&mut self.end_k,"923");
                    field(&mut columns[2],"Step · K",&mut self.step_k,"25");
                });
                ui.columns(3,|columns|{
                    field(&mut columns[0],"Target gas · optional",&mut self.target_gas,"Cl2");
                    field(&mut columns[1],"Other gas activities p/p°",&mut self.other_gas_activities,"SbCl3=1");
                    field(&mut columns[2],"Electron count z · optional",&mut self.electron_count,"2");
                });
                ui.label(RichText::new("Partial pressure uses ideal gas activities. Enter every other gas activity explicitly. E° = −ΔG°/(zF) for the reaction as written.").size(12.0).color(MUTED));
                if primary_button(ui,"Calculate").clicked(){self.result=None;self.error=self.calculate(database).err();}
                if self.result.is_some() && self.result_signature!=self.signature() {
                    self.result=None;
                    ui.colored_label(Color32::from_rgb(150,90,35),"Inputs changed. Calculate again to update the result.");
                }
                if let (Some(profile),Some(workbook_id))=(profile,workbook_id)
                    && let Ok(saved)=profile.thermo_calculations(workbook_id) {
                    ui.collapsing(format!("Saved calculations in this Workbook ({})",saved.len()),|ui|{
                        for item in saved {
                            ui.horizontal(|ui|{
                                ui.label(format!("{} · {}",item.name,item.created_at));
                                if ui.button("Load reaction").clicked(){self.error=self.load_saved(&item.snapshot_json).err();}
                                if ui.button("Inspect snapshot").clicked(){self.selected_snapshot=Some(item.snapshot_json.clone());}
                            });
                        }
                    });
                }
                if let Some(snapshot)=&self.selected_snapshot {
                    ui.collapsing("Saved result and provenance snapshot",|ui|{
                        let display=serde_json::from_str::<serde_json::Value>(snapshot).ok()
                            .and_then(|value|serde_json::to_string_pretty(&value).ok()).unwrap_or_else(||snapshot.clone());
                        ui.code(display);
                    });
                    if ui.button("Close snapshot").clicked(){self.selected_snapshot=None;}
                }
                let Some(series)=self.result.clone() else {return;};
                ui.add_space(15.0);ui.separator();
                ui.heading("Results");
                for change in &series.changes {ui.colored_label(TEAL,format!("{}: {} [{}] → {} [{}] between {} and {} K",change.formula,change.from_phase,change.from_record_id,change.to_phase,change.to_record_id,change.between_k.0,change.between_k.1));}
                egui::ScrollArea::horizontal().id_salt("thermo-results-scroll").show(ui,|ui|{
                egui::Grid::new("thermo-results").striped(true).show(ui,|ui|{
                    for heading in ["T / K","ΔH° / J mol⁻¹","ΔS° / J mol⁻¹ K⁻¹","ΔG° / J mol⁻¹","log₁₀ K","Phases"] {ui.strong(heading);}ui.end_row();
                    for point in series.points.iter().take(200) {
                        ui.label(format!("{:.2}",point.temperature_k));ui.label(format!("{:.3}",point.delta_h_j_mol));
                        ui.label(format!("{:.4}",point.delta_s_j_mol_k));ui.label(format!("{:.3}",point.delta_g_j_mol));
                        ui.label(format!("{:.5}",point.log10_k));
                        ui.label(point.species_used.iter().map(|used|format!("{}({})",used.record.formula,used.record.phase)).collect::<Vec<_>>().join(" · "));
                        ui.end_row();
                    }
                });
                });
                if series.points.len()>200 {ui.label(format!("Showing first 200 of {} points; export CSV for all.",series.points.len()));}
                draw_gibbs_plot(ui,&series);
                if let Some(first)=series.points.first(){
                    match self.gas_pressure(first){Ok(Some((relative,bar)))=>{ui.label(format!("At {:.2} K: p({}) = {:.6e} bar; log₁₀(p/p°) = {:.6}",first.temperature_k,self.target_gas,bar,relative.log10()));},Err(error)=>{ui.colored_label(Color32::DARK_RED,error);},_=>{}}
                    if !self.electron_count.trim().is_empty(){match self.electron_count.trim().parse::<u32>().ok().and_then(|z|standard_electrode_potential(first,z).ok()) {Some(e)=>{ui.label(format!("E° at {:.2} K = {:.6} V",first.temperature_k,e));},None=>{ui.colored_label(Color32::DARK_RED,"Enter a positive integer z for potential.");}}}
                }
                ui.collapsing("Species records and provenance",|ui|{
                    for point in &series.points {
                        ui.strong(format!("{} K",point.temperature_k));
                        for used in &point.species_used {
                            ui.label(format!("{}({}) · {} · {} · revision {} · {} · verified {:?}",used.record.formula,used.record.phase,used.record.source_name,used.record.source_locator,used.record.revision,used.record.source_reference,used.record.verified_at));
                        }
                    }
                });
                ui.horizontal_wrapped(|ui|{
                    if let (Some(profile),Some(workbook_id))=(profile,workbook_id)
                        && ui.button("Save to active Workbook").clicked() {
                        let snapshot=self.snapshot(&series).to_string();
                        match profile.save_thermo_calculation(workbook_id,&self.name,&snapshot){Ok(_)=>{self.notice="Calculation and full record snapshot saved in this Workbook.".into();self.error=None;},Err(e)=>self.error=Some(e.to_string())}
                    }
                    ui.label("CSV path");ui.text_edit_singleline(&mut self.export_path);
                    #[cfg(windows)]
                    if ui.button("Choose CSV…").clicked()
                        && let Some(path)=rfd::FileDialog::new().set_title("Export thermodynamic CSV").add_filter("CSV", &["csv"]).save_file() {
                        self.export_path=path.to_string_lossy().into_owned();
                    }
                    if ui.button("Export CSV").clicked(){
                        if self.export_path.trim().is_empty(){self.error=Some("Enter a CSV destination path.".into());}
                        else {match self.csv(&series).and_then(|csv|std::fs::write(self.export_path.trim(),csv).map_err(|e|e.to_string())) {Ok(())=>{self.notice="CSV exported with source provenance.".into();self.error=None;},Err(e)=>self.error=Some(e)}}
                    }
                });
                ui.collapsing("Excel template export",|ui|{
                    ui.label("Select your existing .xlsx calculation workbook as the template. CrystalForge copies it and adds a results worksheet; original sheets, formulas and formatting remain intact.");
                    field(ui,"Template .xlsx path",&mut self.template_path,"Path to Partial_pressure_Chlorine_Sb-Bi-Te-Se.xlsx");
                    #[cfg(windows)]
                    if ui.button("Choose template…").clicked()
                        && let Some(path)=rfd::FileDialog::new().set_title("Choose your Excel template").add_filter("Excel workbook", &["xlsx"]).pick_file() {
                        self.template_path=path.to_string_lossy().into_owned();
                    }
                    field(ui,"New output .xlsx path",&mut self.excel_path,"Different output path");
                    #[cfg(windows)]
                    if ui.button("Choose output…").clicked()
                        && let Some(path)=rfd::FileDialog::new().set_title("Export Excel workbook").add_filter("Excel workbook", &["xlsx"]).save_file() {
                        self.excel_path=path.to_string_lossy().into_owned();
                    }
                    if ui.button("Export Excel from template").clicked(){
                        let source_name=sources.iter().find(|source|source.id==self.source_id)
                            .map_or(self.source_id.as_str(),|source|source.name.as_str());
                        let export=self.excel_rows(&series).and_then(|rows|{
                            export_template(std::path::Path::new(self.template_path.trim()),std::path::Path::new(self.excel_path.trim()),&self.name,&self.equation,source_name,&rows).map_err(|e|e.to_string())
                        });
                        match export {Ok(())=>{self.notice="Excel workbook exported; original template retained.".into();self.error=None;},Err(error)=>self.error=Some(error)}
                    }
                });
            });
        });
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

fn draw_gibbs_plot(ui: &mut egui::Ui, series: &PhaseSeries) {
    if series.points.len() < 2 {
        return;
    }
    let width = ui.available_width().max(250.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 240.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, Color32::from_rgb(246, 249, 248));
    let inner = egui::Rect::from_min_max(
        rect.min + egui::vec2(70.0, 18.0),
        rect.max - egui::vec2(18.0, 38.0),
    );
    let min_t = series.points.first().unwrap().temperature_k;
    let max_t = series.points.last().unwrap().temperature_k;
    let min_g = series
        .points
        .iter()
        .map(|p| p.delta_g_j_mol)
        .fold(f64::INFINITY, f64::min);
    let max_g = series
        .points
        .iter()
        .map(|p| p.delta_g_j_mol)
        .fold(f64::NEG_INFINITY, f64::max);
    let span_g = (max_g - min_g).max(1.0);
    let pos = |point: &crate::thermodynamics::Calculation| {
        egui::pos2(
            inner.left() + ((point.temperature_k - min_t) / (max_t - min_t)) as f32 * inner.width(),
            inner.bottom() - ((point.delta_g_j_mol - min_g) / span_g) as f32 * inner.height(),
        )
    };
    let axis = egui::Stroke::new(1.0, MUTED);
    painter.line_segment([inner.left_bottom(), inner.right_bottom()], axis);
    painter.line_segment([inner.left_bottom(), inner.left_top()], axis);
    for pair in series.points.windows(2) {
        let same = pair[0]
            .species_used
            .iter()
            .zip(&pair[1].species_used)
            .all(|(a, b)| a.record.id == b.record.id);
        if same {
            painter.line_segment([pos(&pair[0]), pos(&pair[1])], egui::Stroke::new(2.4, TEAL));
        }
    }
    for point in &series.points {
        painter.circle_filled(pos(point), 2.5, TEAL);
    }
    for change in &series.changes {
        let x = inner.left()
            + (((change.between_k.0 + change.between_k.1) / 2.0 - min_t) / (max_t - min_t)) as f32
                * inner.width();
        painter.line_segment(
            [egui::pos2(x, inner.top()), egui::pos2(x, inner.bottom())],
            egui::Stroke::new(1.0, Color32::from_rgb(198, 99, 50)),
        );
    }
    painter.text(
        egui::pos2(inner.left(), rect.bottom() - 23.0),
        egui::Align2::LEFT_CENTER,
        format!("{min_t:.1} K"),
        egui::FontId::proportional(12.0),
        MUTED,
    );
    painter.text(
        egui::pos2(inner.right(), rect.bottom() - 23.0),
        egui::Align2::RIGHT_CENTER,
        format!("{max_t:.1} K"),
        egui::FontId::proportional(12.0),
        MUTED,
    );
    painter.text(
        egui::pos2(rect.left() + 5.0, inner.top()),
        egui::Align2::LEFT_TOP,
        format!("{:.1} kJ/mol", max_g / 1000.0),
        egui::FontId::proportional(11.0),
        MUTED,
    );
    painter.text(
        egui::pos2(rect.left() + 5.0, inner.bottom()),
        egui::Align2::LEFT_BOTTOM,
        format!("{:.1} kJ/mol", min_g / 1000.0),
        egui::FontId::proportional(11.0),
        MUTED,
    );
    ui.label(RichText::new("ΔG° versus temperature · orange markers show phase changes between sampled temperatures").size(11.0).color(MUTED));
}
