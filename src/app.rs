use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::{
    cvt_view::{self, ExperimentAction, ExperimentForm, FormAction},
    editor::Editor,
    model::{CvtExperiment, Entry, Project},
    peak_view::PeakWorkspace,
    profile_view::{self, ProfileForm},
    profiles::{Profile, Profiles},
    store::Store,
};

pub(crate) const INK: Color32 = Color32::from_rgb(29, 43, 59);
pub(crate) const MUTED: Color32 = Color32::from_rgb(100, 116, 132);
pub(crate) const TEAL: Color32 = Color32::from_rgb(20, 112, 109);
const SIDEBAR: Color32 = Color32::from_rgb(26, 40, 55);
pub(crate) const PAPER: Color32 = Color32::from_rgb(249, 250, 248);

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Experiments,
    Tools,
    Logbook,
    RecentlyDeleted,
}

#[derive(Clone)]
enum Action {
    SelectTab(Tab),
    OpenProfiles,
    NewProfile,
    EditProfile,
    SaveProfile,
    SwitchProfile(String),
    NewExperiment,
    OpenExperiment(String),
    PreviousLogs,
    CreateExperiment,
    SelectProject(String),
    OpenEntry(String),
    NewEntry,
    NewProject,
    Reload,
    Close,
    Save,
    Filter,
    CreateProject,
    DeleteExperiment(String),
    DeleteEntry(Entry),
    DeleteLog,
    ConfirmDelete,
    RestoreExperiment(String),
    RestoreEntry(Entry),
}

#[derive(Clone)]
enum Deletion {
    Experiment(CvtExperiment),
    Entry(Entry),
    Log {
        project_id: String,
        name: String,
        entries: Vec<Entry>,
    },
}

impl Deletion {
    fn affects_editor(&self, editor: &Editor) -> bool {
        match self {
            Self::Experiment(experiment) => experiment.log_project_id == editor.project_id,
            Self::Entry(entry) => editor
                .saved
                .as_ref()
                .is_some_and(|saved| saved.id == entry.id),
            Self::Log { project_id, .. } => *project_id == editor.project_id,
        }
    }
}

#[derive(Default)]
struct ProjectForm {
    name: String,
    description: String,
}

pub struct ResearchMinionApp {
    store: Option<Store>,
    registry: Option<Profiles>,
    active_profile: Option<Profile>,
    profile_list: Vec<Profile>,
    profile_form: Option<ProfileForm>,
    profile_menu: bool,
    profile_texture: Option<egui::TextureHandle>,
    legacy_pending: bool,
    startup_error: Option<String>,
    tab: Tab,
    experiments: Vec<CvtExperiment>,
    experiment_form: Option<ExperimentForm>,
    active_experiment: Option<CvtExperiment>,
    projects: Vec<Project>,
    entries: Vec<Entry>,
    project_id: Option<String>,
    editor: Option<Editor>,
    search: String,
    project_form: Option<ProjectForm>,
    pending: Option<Action>,
    error: Option<String>,
    notice: String,
    allow_close: bool,
    deletion: Option<Deletion>,
    deleted_experiments: Vec<CvtExperiment>,
    deleted_entries: Vec<Entry>,
    peak: PeakWorkspace,
    peak_drafts: std::collections::BTreeMap<String, PeakWorkspace>,
}

impl ResearchMinionApp {
    pub fn new(cc: &eframe::CreationContext<'_>, registry: Result<Profiles, String>) -> Self {
        configure_style(&cc.egui_ctx);
        let (registry, startup_error) = match registry {
            Ok(registry) => (Some(registry), None),
            Err(error) => (None, Some(error)),
        };
        let mut app = Self {
            store: None,
            registry,
            active_profile: None,
            profile_list: vec![],
            profile_form: None,
            profile_menu: false,
            profile_texture: None,
            legacy_pending: false,
            startup_error,
            tab: Tab::default(),
            experiments: vec![],
            experiment_form: None,
            active_experiment: None,
            projects: vec![],
            entries: vec![],
            project_id: None,
            editor: None,
            search: String::new(),
            project_form: None,
            pending: None,
            error: None,
            notice: "Offline research workspace".into(),
            allow_close: false,
            deletion: None,
            deleted_experiments: vec![],
            deleted_entries: vec![],
            peak: PeakWorkspace::default(),
            peak_drafts: std::collections::BTreeMap::new(),
        };
        let initial = app.registry.as_ref().map(|registry| {
            Ok::<_, String>((
                registry.all().map_err(|error| error.to_string())?,
                registry.active().map_err(|error| error.to_string())?,
                registry
                    .has_unassigned_workspace()
                    .map_err(|error| error.to_string())?,
            ))
        });
        match initial {
            Some(Ok((profiles, active, legacy))) => {
                app.profile_list = profiles;
                app.legacy_pending = legacy;
                if let Some(profile) = active {
                    if !app.activate_profile(&profile.id, &cc.egui_ctx) {
                        app.profile_form = Some(ProfileForm::default());
                    }
                } else {
                    app.profile_form = Some(ProfileForm::default());
                }
            }
            Some(Err(error)) => app.startup_error = Some(error),
            None => {}
        }
        app
    }

    fn activate_profile(&mut self, id: &str, ctx: &egui::Context) -> bool {
        let result = self
            .registry
            .as_ref()
            .ok_or("Profile catalog unavailable".to_string())
            .and_then(|registry| {
                let profile = registry.profile(id).map_err(|error| error.to_string())?;
                let store = registry.workspace(id).map_err(|error| error.to_string())?;
                let projects = store.legacy_projects().map_err(|error| error.to_string())?;
                let experiments = store.cvt_experiments().map_err(|error| error.to_string())?;
                registry.activate(id).map_err(|error| error.to_string())?;
                Ok((profile, store, projects, experiments))
            });
        match result {
            Ok((profile, store, projects, experiments)) => {
                self.profile_texture = profile
                    .picture_png
                    .as_ref()
                    .and_then(|bytes| profile_view::avatar_texture(ctx, &profile.id, bytes));
                self.notice = format!("Active profile: {}", profile.full_name);
                if let Some(previous) = &self.active_profile {
                    self.peak_drafts
                        .insert(previous.id.clone(), std::mem::take(&mut self.peak));
                }
                self.peak = self.peak_drafts.remove(&profile.id).unwrap_or_default();
                self.active_profile = Some(profile);
                self.store = Some(store);
                self.projects = projects;
                self.experiments = experiments;
                self.entries.clear();
                self.editor = None;
                self.project_id = None;
                self.active_experiment = None;
                self.search.clear();
                self.deleted_entries.clear();
                self.deleted_experiments.clear();
                self.experiment_form = None;
                self.project_form = None;
                self.deletion = None;
                self.tab = Tab::Experiments;
                self.error = None;
                if let Some(project) = self.projects.first() {
                    self.select_project(project.id.clone());
                }
                ctx.request_repaint();
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    fn save_profile(&mut self, ctx: &egui::Context) -> bool {
        let (Some(registry), Some(form)) = (&mut self.registry, &mut self.profile_form) else {
            return false;
        };
        if let Err(error) = form.resolve_picture() {
            self.error = Some(error);
            return false;
        }
        let editing = form.id.as_ref().is_some_and(|id| {
            self.active_profile
                .as_ref()
                .is_some_and(|profile| &profile.id == id)
        });
        let result = match &form.id {
            Some(id) => registry.update(id, &form.full_name, form.picture_png.as_deref()),
            None => registry.create(&form.full_name, form.picture_png.as_deref()),
        };
        match result {
            Ok(profile) => {
                if let Some(form) = &mut self.profile_form {
                    form.id = Some(profile.id.clone());
                }
                match registry.all() {
                    Ok(profiles) => self.profile_list = profiles,
                    Err(error) => {
                        self.error = Some(error.to_string());
                        return false;
                    }
                }
                if editing {
                    self.profile_texture = profile
                        .picture_png
                        .as_ref()
                        .and_then(|bytes| profile_view::avatar_texture(ctx, &profile.id, bytes));
                    self.notice = format!("Active profile: {}", profile.full_name);
                    self.active_profile = Some(profile);
                } else if !self.activate_profile(&profile.id, ctx) {
                    return false;
                }
                self.profile_form = None;
                self.profile_menu = false;
                self.legacy_pending = false;
                self.error = None;
                ctx.request_repaint();
                true
            }
            Err(error) => {
                self.error = Some(error.to_string());
                false
            }
        }
    }

    fn refresh_entries(&mut self) -> bool {
        if let (Some(store), Some(id)) = (&self.store, &self.project_id) {
            match store.entries(id, &self.search) {
                Ok(entries) => {
                    self.entries = entries;
                    return true;
                }
                Err(error) => {
                    self.entries.clear();
                    self.error = Some(error.to_string());
                }
            }
        }
        false
    }

    fn select_project(&mut self, id: String) {
        self.search.clear();
        self.project_id = Some(id.clone());
        self.error = None;
        if !self.refresh_entries() {
            self.editor = None;
            return;
        }
        self.editor = Some(match self.entries.first() {
            Some(entry) => Editor::from_entry(entry.clone()),
            None => Editor::new(id),
        });
    }

    fn save(&mut self) -> bool {
        let (Some(editor), Some(store)) = (&mut self.editor, &self.store) else {
            return false;
        };
        match editor.save(store) {
            Ok(()) => {
                self.notice = "Saved to this device".into();
                self.error = None;
                self.refresh_entries();
                true
            }
            Err(error) => {
                self.error = Some(error.to_string());
                false
            }
        }
    }

    fn refresh_experiments(&mut self) {
        if let Some(store) = &self.store {
            match store.cvt_experiments() {
                Ok(experiments) => self.experiments = experiments,
                Err(error) => self.error = Some(error.to_string()),
            }
        }
    }

    fn create_experiment(&mut self) -> bool {
        let (Some(store), Some(form)) = (&self.store, &self.experiment_form) else {
            return false;
        };
        let result = form
            .conditions()
            .map_err(|error| error.to_string())
            .and_then(|conditions| {
                let mut details = form.details.clone();
                details.experiment_date = form.date()?;
                store
                    .create_cvt_experiment_with_details(&form.name, conditions, &details)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(_) => {
                self.experiment_form = None;
                self.error = None;
                self.notice = "Experiment saved to this device".into();
                self.refresh_experiments();
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    fn has_unsaved_changes(&self) -> bool {
        self.profile_form
            .as_ref()
            .is_some_and(ProfileForm::is_dirty)
            || self.editor.as_ref().is_some_and(Editor::is_dirty)
            || self
                .experiment_form
                .as_ref()
                .is_some_and(ExperimentForm::is_dirty)
    }

    fn save_active_changes(&mut self, ctx: &egui::Context) -> bool {
        if self.profile_form.is_some() {
            self.save_profile(ctx)
        } else if self.experiment_form.is_some() {
            self.create_experiment()
        } else {
            self.save()
        }
    }

    fn discard_changes(&mut self) {
        self.profile_form = None;
        self.experiment_form = None;
        if let Some(editor) = &mut self.editor {
            *editor = match &editor.saved {
                Some(entry) => Editor::from_entry(entry.clone()),
                None => Editor::new(editor.project_id.clone()),
            };
        }
        self.error = None;
    }

    fn request(&mut self, action: Action, ctx: &egui::Context) {
        // Actions are applied after drawing; schedule their new view immediately.
        ctx.request_repaint();
        let needs_guard = matches!(
            action,
            Action::SelectTab(_)
                | Action::SwitchProfile(_)
                | Action::NewProfile
                | Action::EditProfile
                | Action::NewExperiment
                | Action::OpenExperiment(_)
                | Action::PreviousLogs
                | Action::SelectProject(_)
                | Action::OpenEntry(_)
                | Action::NewEntry
                | Action::NewProject
                | Action::Reload
                | Action::Close
        );
        if needs_guard && self.has_unsaved_changes() {
            self.pending = Some(action);
        } else {
            self.apply(action, ctx);
        }
    }

    fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::OpenProfiles => {
                if let Some(registry) = &self.registry {
                    match registry.all() {
                        Ok(profiles) => {
                            self.profile_list = profiles;
                            self.profile_menu = true;
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            }
            Action::NewProfile => {
                self.profile_menu = false;
                self.profile_form = Some(ProfileForm::default());
                self.error = None;
            }
            Action::EditProfile => {
                self.profile_menu = false;
                self.profile_form = Some(ProfileForm::new(self.active_profile.as_ref()));
                self.error = None;
            }
            Action::SaveProfile => {
                self.save_profile(ctx);
            }
            Action::SwitchProfile(id) => {
                self.profile_menu = false;
                if self.activate_profile(&id, ctx) {
                    self.profile_form = None;
                }
            }
            Action::SelectTab(tab) => {
                self.tab = tab;
                self.error = None;
                if tab == Tab::Experiments || tab == Tab::Tools {
                    self.refresh_experiments();
                }
                if tab == Tab::RecentlyDeleted {
                    self.refresh_deleted();
                }
            }
            Action::OpenExperiment(id) => {
                let result = self.store.as_ref().map(|store| store.cvt_experiment(&id));
                match result {
                    Some(Ok(experiment)) => {
                        let project_id = experiment.log_project_id.clone();
                        self.active_experiment = Some(experiment);
                        self.select_project(project_id);
                        self.tab = Tab::Logbook;
                    }
                    Some(Err(error)) => self.error = Some(error.to_string()),
                    None => {}
                }
            }
            Action::PreviousLogs => {
                self.active_experiment = None;
                self.tab = Tab::Logbook;
                if let Some(project) = self.projects.first() {
                    self.select_project(project.id.clone());
                } else {
                    self.project_id = None;
                    self.editor = None;
                    self.entries.clear();
                }
            }
            Action::NewExperiment => {
                self.error = None;
                self.experiment_form = Some(ExperimentForm::default());
            }
            Action::CreateExperiment => {
                self.create_experiment();
            }
            Action::Save => {
                self.save();
            }
            Action::Filter => {
                self.refresh_entries();
            }
            Action::SelectProject(id) => {
                self.active_experiment = None;
                self.select_project(id);
            }
            Action::NewEntry => {
                if let Some(id) = &self.project_id {
                    self.editor = Some(Editor::new(id.clone()));
                    self.error = None;
                }
            }
            Action::OpenEntry(id) => self.load_entry(&id),
            Action::Reload => {
                if let Some(id) = self
                    .editor
                    .as_ref()
                    .and_then(|e| e.saved.as_ref())
                    .map(|e| e.id.clone())
                {
                    self.load_entry(&id);
                    self.refresh_entries();
                }
            }
            Action::NewProject => self.project_form = Some(ProjectForm::default()),
            Action::CreateProject => {
                if let (Some(store), Some(form)) = (&self.store, &self.project_form) {
                    match store.create_project(&form.name, &form.description) {
                        Ok(project) => {
                            let id = project.id.clone();
                            self.projects.push(project);
                            self.project_form = None;
                            self.select_project(id);
                            self.notice = "Project created · write your first entry".into();
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            }
            Action::DeleteExperiment(id) => {
                match self.store.as_ref().map(|store| store.cvt_experiment(&id)) {
                    Some(Ok(experiment)) => {
                        self.deletion = Some(Deletion::Experiment(experiment));
                        self.error = None;
                    }
                    Some(Err(error)) => self.error = Some(error.to_string()),
                    None => {}
                }
            }
            Action::DeleteEntry(entry) => {
                self.deletion = Some(Deletion::Entry(entry));
                self.error = None;
            }
            Action::DeleteLog => {
                if let (Some(store), Some(id)) = (&self.store, &self.project_id) {
                    match store.entries(id, "") {
                        Ok(entries) => {
                            let name = self
                                .active_experiment
                                .as_ref()
                                .map(|experiment| experiment.name.clone())
                                .or_else(|| {
                                    self.projects
                                        .iter()
                                        .find(|project| &project.id == id)
                                        .map(|project| project.name.clone())
                                })
                                .unwrap_or_else(|| "Current log".into());
                            self.deletion = Some(Deletion::Log {
                                project_id: id.clone(),
                                name,
                                entries,
                            });
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            }
            Action::ConfirmDelete => self.confirm_delete(),
            Action::RestoreExperiment(id) => {
                if let Some(store) = &self.store {
                    match store.restore_experiment(&id) {
                        Ok(()) => {
                            self.notice = "Experiment and its log restored".into();
                            self.error = None;
                            self.refresh_deleted();
                            self.refresh_experiments();
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            }
            Action::RestoreEntry(entry) => {
                if let Some(store) = &self.store {
                    match store.restore_entry(&entry.id, entry.revision) {
                        Ok(()) => {
                            self.notice = "Log entry restored".into();
                            self.error = None;
                            self.refresh_deleted();
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            }
            Action::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn refresh_deleted(&mut self) {
        if let Some(store) = &self.store {
            match (store.deleted_experiments(), store.deleted_entries()) {
                (Ok(experiments), Ok(entries)) => {
                    self.deleted_experiments = experiments;
                    self.deleted_entries = entries;
                }
                (Err(error), _) | (_, Err(error)) => self.error = Some(error.to_string()),
            }
        }
    }

    fn confirm_delete(&mut self) {
        let (Some(store), Some(target)) = (&self.store, self.deletion.clone()) else {
            return;
        };
        let replaces_editor = self
            .editor
            .as_ref()
            .is_some_and(|editor| target.affects_editor(editor));
        let result = match &target {
            Deletion::Experiment(experiment) => store.delete_experiment(&experiment.id),
            Deletion::Entry(entry) => store.delete_entry(&entry.id, entry.revision),
            Deletion::Log {
                project_id,
                entries,
                ..
            } => {
                let expected: Vec<_> = entries
                    .iter()
                    .map(|entry| (entry.id.clone(), entry.revision))
                    .collect();
                store.delete_log_entries(project_id, &expected).map(|_| ())
            }
        };
        match result {
            Ok(()) => {
                self.deletion = None;
                self.error = None;
                self.notice = "Moved to Recently deleted · restore from CVT Experiments".into();
                match target {
                    Deletion::Experiment(experiment) => {
                        if self
                            .active_experiment
                            .as_ref()
                            .is_some_and(|active| active.id == experiment.id)
                        {
                            self.active_experiment = None;
                            self.editor = None;
                            self.project_id = None;
                            self.entries.clear();
                        }
                        self.tab = Tab::Experiments;
                        self.refresh_experiments();
                    }
                    Deletion::Entry(_) | Deletion::Log { .. } => {
                        self.refresh_entries();
                        if replaces_editor {
                            self.editor = self
                                .entries
                                .first()
                                .cloned()
                                .map(Editor::from_entry)
                                .or_else(|| self.project_id.clone().map(Editor::new));
                        }
                    }
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn recently_deleted(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        egui::ScrollArea::vertical().show(ui,|ui| {
            egui::Frame::new().inner_margin(24).show(ui,|ui| {
                if primary_button(ui,"Back to experiments").clicked() { actions.push(Action::SelectTab(Tab::Experiments)); }
                ui.add_space(14.0);
                ui.heading("Recently deleted");
                ui.label("Saved records are retained locally until restored. There is no automatic expiry.");
                ui.add_space(18.0);
                ui.heading("Experiments");
                if self.deleted_experiments.is_empty() {ui.label("No deleted experiments.");}
                for experiment in &self.deleted_experiments {
                    ui.push_id(&experiment.id,|ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Restore experiment and log").clicked() {actions.push(Action::RestoreExperiment(experiment.id.clone()));}
                            ui.strong(&experiment.name);
                            ui.label(format!("Recorded {} UTC",experiment.created_at.format("%Y-%m-%d")));
                        });
                    });
                }
                ui.add_space(18.0);
                ui.heading("Log entries");
                ui.label("Entries from a deleted experiment appear here after that experiment is restored.");
                if self.deleted_entries.is_empty() {ui.label("No deleted entries in active logs.");}
                for entry in &self.deleted_entries {
                    ui.push_id(&entry.id,|ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Restore entry").clicked() {actions.push(Action::RestoreEntry(entry.clone()));}
                            ui.strong(&entry.title);
                            let owner=self.experiments.iter().find(|experiment|experiment.log_project_id==entry.project_id).map(|experiment|experiment.name.as_str())
                                .or_else(||self.projects.iter().find(|project|project.id==entry.project_id).map(|project|project.name.as_str())).unwrap_or("Previous log");
                            ui.label(format!("Event {} · {}",entry.event_date,owner));
                        });
                    });
                }
            });
        });
    }

    fn load_entry(&mut self, id: &str) {
        if let Some(store) = &self.store {
            match store.entry(id) {
                Ok(entry) => {
                    self.editor = Some(Editor::from_entry(entry));
                    self.error = None;
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
    }

    fn sidebar(&self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        egui::SidePanel::left("projects")
            .exact_width(224.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(SIDEBAR).inner_margin(20))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(
                    RichText::new("RESEARCH")
                        .size(13.0)
                        .color(Color32::from_rgb(125, 185, 180)),
                );
                ui.label(
                    RichText::new("Minion")
                        .size(32.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.add_space(7.0);
                ui.label(
                    RichText::new("Your work, remembered.")
                        .size(12.0)
                        .color(Color32::LIGHT_GRAY),
                );
                ui.add_space(24.0);
                if let Some(profile) = &self.active_profile {
                    ui.horizontal(|ui| {
                        profile_view::avatar(
                            ui,
                            &profile.full_name,
                            self.profile_texture.as_ref(),
                            48.0,
                        );
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&profile.full_name)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .truncate(),
                            )
                            .on_hover_text(&profile.full_name);
                            if ui.button("Profiles…").clicked() {
                                actions.push(Action::OpenProfiles);
                            }
                        });
                    });
                }
                ui.add_space(24.0);
                ui.label(
                    RichText::new("WORKSPACE")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_gray(160)),
                );
                ui.add_space(8.0);
                for (tab, label) in [
                    (Tab::Experiments, "CVT Experiments"),
                    (Tab::Tools, "Scientific Utilities"),
                ] {
                    let selected = self.tab == tab
                        || (tab == Tab::Experiments
                            && matches!(self.tab, Tab::Logbook | Tab::RecentlyDeleted));
                    if ui
                        .add(
                            egui::Button::new(RichText::new(label).color(Color32::WHITE))
                                .fill(if selected { TEAL } else { SIDEBAR })
                                .corner_radius(6.0)
                                .min_size(Vec2::new(ui.available_width(), 40.0)),
                        )
                        .clicked()
                        && self.tab != tab
                    {
                        actions.push(Action::SelectTab(tab));
                    }
                    ui.add_space(4.0);
                }
                ui.add_space(24.0);
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("OFFLINE BY DESIGN")
                            .size(10.0)
                            .strong()
                            .color(Color32::from_rgb(125, 185, 180)),
                    );
                    ui.label(
                        RichText::new(concat!("Research Minion ", env!("CARGO_PKG_VERSION")))
                            .size(11.0)
                            .color(Color32::from_gray(150)),
                    );
                });
            });
    }

    fn workspace(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        egui::Frame::new()
            .fill(PAPER)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    if primary_button(ui, "Back to experiments").clicked() {
                        actions.push(Action::SelectTab(Tab::Experiments));
                    }
                    if self.project_id.is_some() && ui.button("+ New log entry").clicked() {
                        actions.push(Action::NewEntry);
                    }
                });
            });
        ui.separator();
        egui::SidePanel::right("experiment-details")
            .default_width(260.0)
            .width_range(220.0..=340.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(PAPER).inner_margin(18))
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("experiment-details-scroll")
                    .show(ui, |ui| {
                        if let Some(experiment) = &self.active_experiment {
                            ui.label(RichText::new("EXPERIMENT").size(11.0).strong().color(MUTED));
                            ui.heading(RichText::new(&experiment.name).size(23.0).color(INK));
                            ui.add_space(14.0);
                            crate::materials_view::experiment_summary(ui, experiment);
                            ui.add_space(20.0);
                            if ui.button("Delete experiment…").clicked() {
                                actions.push(Action::DeleteExperiment(experiment.id.clone()));
                            }
                        } else {
                            ui.heading("Previous project logs");
                            let project = self
                                .projects
                                .iter()
                                .find(|project| Some(&project.id) == self.project_id.as_ref());
                            egui::ComboBox::from_id_salt("previous-project")
                                .selected_text(
                                    project
                                        .map_or("Choose project", |project| project.name.as_str()),
                                )
                                .show_ui(ui, |ui| {
                                    for project in &self.projects {
                                        if ui
                                            .selectable_label(
                                                Some(&project.id) == self.project_id.as_ref(),
                                                &project.name,
                                            )
                                            .clicked()
                                        {
                                            actions.push(Action::SelectProject(project.id.clone()));
                                        }
                                    }
                                });
                            if ui.button("New project").clicked() {
                                actions.push(Action::NewProject);
                            }
                            if let Some(project) = project {
                                ui.label(&project.description);
                            }
                        }
                    });
            });
        egui::SidePanel::left("entry-list").exact_width(228.0).resizable(false)
            .frame(egui::Frame::new().fill(PAPER).inner_margin(16))
            .show_inside(ui, |ui| {
                ui.label(RichText::new("Notes").size(18.0).strong().color(INK));
                ui.add_space(12.0);
                if ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search title or notes…").desired_width(f32::INFINITY)).changed() {
                    actions.push(Action::Filter);
                }
                ui.add_space(5.0);
                ui.label(RichText::new(format!("{} notes", self.entries.len())).size(12.0).color(MUTED));
                if ui.add_enabled(!self.entries.is_empty(), egui::Button::new("Delete all notes…"))
                    .on_hover_text("Clear every saved note in this log, including notes hidden by search.")
                    .clicked() { actions.push(Action::DeleteLog); }
                ui.add_space(12.0);
                egui::ScrollArea::vertical().id_salt("entry-list-scroll").show(ui, |ui| {
                    if self.entries.is_empty() {
                        ui.label(RichText::new(if self.search.is_empty() { "No saved entries yet. Your first entry starts on the right." } else { "No matches in this project. Try another term or clear the search." }).color(MUTED));
                    }
                    for entry in &self.entries {
                        let selected = self.editor.as_ref().and_then(|e| e.saved.as_ref()).is_some_and(|e| e.id == entry.id);
                        ui.push_id(&entry.id, |ui| {
                            egui::Frame::new()
                                .fill(if selected { Color32::from_rgb(224, 239, 234) } else { Color32::WHITE })
                                .stroke(Stroke::new(1.0, if selected { Color32::from_rgb(125, 178, 165) } else { Color32::from_gray(214) }))
                                .corner_radius(7.0).inner_margin(12)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    let open = ui.vertical(|ui| {
                                        ui.label(RichText::new(entry.event_date.to_string()).size(12.0).color(TEAL));
                                        ui.label(RichText::new(&entry.title).strong().color(INK));
                                        let preview: String = entry.body.lines().find(|s| !s.trim().is_empty()).unwrap_or("No notes").chars().take(55).collect();
                                        ui.label(RichText::new(preview).size(12.0).color(MUTED));
                                    }).response.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if open.clicked() && !selected { actions.push(Action::OpenEntry(entry.id.clone())); }
                                    ui.add_space(6.0);
                                    if ui.button("Delete note…").on_hover_text("Delete only this saved note").clicked() {
                                        actions.push(Action::DeleteEntry(entry.clone()));
                                    }
                                });
                        });
                        ui.add_space(8.0);
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(18))
            .show_inside(ui, |ui| {
                if let Some(editor) = &mut self.editor {
                    egui::ScrollArea::vertical()
                        .id_salt("entry-editor-scroll")
                        .show_viewport(ui, |ui, viewport| {
                            draw_editor(ui, editor, actions, viewport.height());
                        });
                }
            });
    }

    fn dialogs(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        // Egui modal layers can retain their foreground order across frames.
        // Render only the confirmation while navigating, retaining form state.
        if self.pending.is_none() && self.deletion.is_none() {
            if self.profile_menu {
                let mut close = false;
                egui::Modal::new(egui::Id::new("profiles-menu")).show(ctx, |ui| {
                    ui.set_width(480.0);
                    ui.heading("User profiles");
                    ui.label(
                        RichText::new("Each profile has its own experiments and notes.")
                            .color(MUTED),
                    );
                    ui.add_space(14.0);
                    egui::ScrollArea::vertical()
                        .max_height((ctx.content_rect().height() - 240.0).max(140.0))
                        .show(ui, |ui| {
                            for profile in &self.profile_list {
                                ui.push_id(&profile.id, |ui| {
                                    ui.horizontal(|ui| {
                                        profile_view::avatar(ui, &profile.full_name, None, 36.0);
                                        ui.add_sized(
                                            [240.0, 36.0],
                                            egui::Label::new(
                                                RichText::new(&profile.full_name).strong(),
                                            )
                                            .truncate(),
                                        )
                                        .on_hover_text(&profile.full_name);
                                        if self
                                            .active_profile
                                            .as_ref()
                                            .is_some_and(|active| active.id == profile.id)
                                        {
                                            ui.label(RichText::new("Active").color(TEAL));
                                        } else if ui.button("Switch").clicked() {
                                            actions.push(Action::SwitchProfile(profile.id.clone()));
                                            close = true;
                                        }
                                    });
                                });
                            }
                        });
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if primary_button(ui, "New profile").clicked() {
                            actions.push(Action::NewProfile);
                            close = true;
                        }
                        if self.active_profile.is_some() && ui.button("Edit my profile").clicked() {
                            actions.push(Action::EditProfile);
                            close = true;
                        }
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
                if close {
                    self.profile_menu = false;
                    ctx.request_repaint();
                }
            } else if self.active_profile.is_some()
                && let Some(form) = &mut self.profile_form
            {
                let mut outcome = None;
                egui::Modal::new(egui::Id::new("edit-profile")).show(ctx, |ui| {
                    ui.set_width(500.0);
                    ui.heading(if form.id.is_some() {
                        "Edit profile"
                    } else {
                        "Create profile"
                    });
                    outcome = form.show(ui, self.error.as_deref(), true);
                });
                match outcome {
                    Some(true) => actions.push(Action::SaveProfile),
                    Some(false) => {
                        self.profile_form = None;
                        self.error = None;
                        ctx.request_repaint();
                    }
                    None => {}
                }
            }
            if let Some(form) = &mut self.experiment_form {
                match form.show(ctx, self.error.as_deref()) {
                    Some(FormAction::Save) => actions.push(Action::CreateExperiment),
                    Some(FormAction::Changed) => {
                        self.error = None;
                        ctx.request_repaint();
                    }
                    Some(FormAction::Cancel) => {
                        self.experiment_form = None;
                        self.error = None;
                        ctx.request_repaint();
                    }
                    None => {}
                }
            }
            if let Some(form) = &mut self.project_form {
                let mut cancel = false;
                egui::Modal::new(egui::Id::new("new-project")).show(ctx, |ui| {
                    ui.set_width(420.0);
                    ui.heading("Create a project");
                    ui.label(
                        RichText::new("A project brings related research entries together.")
                            .color(MUTED),
                    );
                    ui.add_space(16.0);
                    ui.label("Project name");
                    ui.add(
                        egui::TextEdit::singleline(&mut form.name)
                            .hint_text("e.g. Sb₂Se₃ crystal growth")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(10.0);
                    ui.label("Description (optional)");
                    ui.add(
                        egui::TextEdit::multiline(&mut form.description)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    );
                    if let Some(error) = &self.error {
                        ui.colored_label(Color32::DARK_RED, error);
                    }
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if primary_button(ui, "Create project").clicked() {
                            actions.push(Action::CreateProject);
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if cancel {
                    self.project_form = None;
                    self.error = None;
                }
            }
        }
        if self.pending.is_none()
            && let Some(target) = &self.deletion
        {
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("confirm-deletion")).show(ctx,|ui| {
                ui.set_width(480.0);
                ui.heading(match target { Deletion::Experiment(_) => "Delete experiment?", Deletion::Entry(_) => "Delete this note?", Deletion::Log {..} => "Delete all notes in this log?" });
                match target {
                    Deletion::Experiment(experiment) => {
                        ui.strong(&experiment.name);
                        ui.label("This hides the experiment and its entire log. Restore both from Recently deleted on the Experiments page.");
                    }
                    Deletion::Entry(entry) => {
                        ui.strong(&entry.title);
                        ui.label("Only this note will be deleted. Other notes stay in this log. You can restore it from Recently deleted.");
                    }                    Deletion::Log {name,entries,..} => {
                        ui.strong(name);
                        ui.label(format!("Move all {} saved log entries to Recently deleted? The experiment remains. Restore entries individually from Recently deleted.",entries.len()));
                    }

                }
                if self.editor.as_ref().is_some_and(|editor| editor.is_dirty() && target.affects_editor(editor)) {
                    ui.colored_label(Color32::DARK_RED,"Unsaved edits to this note will be discarded if you delete. Cancel to keep editing.");
                }
                if let Some(error)=&self.error {ui.colored_label(Color32::DARK_RED,error);}
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {actions.push(Action::ConfirmDelete);}
                    if primary_button(ui,"Cancel").clicked() {cancel=true;}
                });
            });
            if cancel {
                self.deletion = None;
                self.error = None;
                ctx.request_repaint();
            }
        }
        if self.pending.is_some() {
            let mut decision = None;
            egui::Modal::new(egui::Id::new("unsaved-entry")).show(ctx, |ui| {
                ui.set_width(450.0);
                ui.heading("Keep your changes?");
                ui.label(if self.profile_form.is_some() {
                    "This profile has unsaved changes. Save it before continuing, or discard it."
                } else if self.experiment_form.is_some() {
                    "This new experiment has not been saved. Save it before continuing, or discard it."
                } else {
                    "This entry has unsaved changes. Save them before continuing, or discard the changes."
                });
                if let Some(error) = &self.error { ui.colored_label(Color32::DARK_RED, error); }
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    if primary_button(ui, "Save and continue").clicked() { decision = Some(0); }
                    if ui.button("Discard changes").clicked() { decision = Some(1); }
                    if ui.button("Keep editing").clicked() { decision = Some(2); }
                });
            });
            if decision.is_some() {
                ctx.request_repaint();
            }
            match decision {
                Some(0) if self.save_active_changes(ctx) => {
                    if let Some(action) = self.pending.take() {
                        self.apply(action, ctx);
                    }
                }
                Some(1) => {
                    self.discard_changes();
                    if let Some(action) = self.pending.take() {
                        self.apply(action, ctx);
                    }
                }
                Some(2) => {
                    self.pending = None;
                }
                _ => {}
            }
        }
    }
}

impl eframe::App for ResearchMinionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(error) = &self.startup_error {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(40.0);
                ui.heading("Research Minion could not open its local workspace");
                ui.add_space(12.0);
                ui.label(error);
                ui.add_space(18.0);
                ui.label("Your existing data has not been replaced. Check folder permissions, then restart.");
                ui.label("You can choose a different local folder with --data-dir <folder>.");
            });
            return;
        }
        let mut actions = Vec::new();
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.has_unsaved_changes()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.pending.is_none() {
                actions.push(Action::Close);
            }
        }
        if self.active_profile.is_none() {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(PAPER).inner_margin(32)).show(ctx,|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(25.0);
                    ui.heading(RichText::new("Welcome to Research Minion").size(28.0).color(INK));
                    ui.label("Create your offline user profile to start.");
                    ui.add_space(20.0);
                    if self.pending.is_none() && !self.profile_menu {
                        ui.allocate_ui_with_layout(egui::vec2(520.0,460.0),egui::Layout::top_down(egui::Align::LEFT),|ui| {
                            if self.legacy_pending {ui.label(RichText::new("Your existing experiments and notes will belong to this first profile.").color(TEAL));ui.add_space(12.0);}
                            if let Some(form)=&mut self.profile_form
                                && form.show(ui,self.error.as_deref(),false)==Some(true){actions.push(Action::SaveProfile);}
                            if !self.profile_list.is_empty() && ui.button("Choose an existing profile").clicked(){actions.push(Action::OpenProfiles);}
                        });
                    }
                });
            });
            self.dialogs(ctx, &mut actions);
            for action in actions {
                self.request(action, ctx);
            }
            return;
        }
        if self.pending.is_none()
            && self.deletion.is_none()
            && self.project_form.is_none()
            && self.profile_form.is_none()
            && !self.profile_menu
            && self.experiment_form.is_none()
            && self.tab == Tab::Logbook
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
        {
            actions.push(Action::Save);
        }
        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::new().fill(PAPER).inner_margin(8))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.notice).size(11.0).color(TEAL));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(store) = &self.store {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(store.path().display().to_string())
                                        .size(10.0)
                                        .color(MUTED),
                                )
                                .truncate(),
                            )
                            .on_hover_text(store.path().display().to_string());
                        }
                    });
                });
            });
        self.sidebar(ctx, &mut actions);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::WHITE))
            .show(ctx, |ui| {
                if let Some(error) = &self.error {
                    let mut dismiss = false;
                    egui::Frame::new()
                        .fill(Color32::from_rgb(255, 237, 233))
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(Color32::DARK_RED, error);
                                dismiss = ui.small_button("Dismiss").clicked();
                            });
                        });
                    if dismiss {
                        self.error = None;
                    }
                }
                match self.tab {
                    Tab::Experiments => {
                        match cvt_view::experiments(
                            ui,
                            &self.experiments,
                            !self.projects.is_empty(),
                        ) {
                            Some(ExperimentAction::New) => actions.push(Action::NewExperiment),
                            Some(ExperimentAction::Open(id)) => {
                                actions.push(Action::OpenExperiment(id))
                            }
                            Some(ExperimentAction::PreviousLogs) => {
                                actions.push(Action::PreviousLogs)
                            }
                            Some(ExperimentAction::Delete(id)) => {
                                actions.push(Action::DeleteExperiment(id))
                            }
                            Some(ExperimentAction::RecentlyDeleted) => {
                                actions.push(Action::SelectTab(Tab::RecentlyDeleted))
                            }
                            None => {}
                        }
                    }
                    Tab::Tools => self.peak.show(ui, self.store.as_ref(), &self.experiments),
                    Tab::RecentlyDeleted => self.recently_deleted(ui, &mut actions),
                    Tab::Logbook => self.workspace(ui, &mut actions),
                }
            });
        self.dialogs(ctx, &mut actions);
        for action in actions {
            self.request(action, ctx);
        }
    }
}

fn draw_editor(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    actions: &mut Vec<Action>,
    viewport_height: f32,
) {
    let content_top = ui.cursor().top();
    ui.horizontal_wrapped(|ui| {
        if primary_button(ui, "Save entry")
            .on_hover_text("Save locally · Ctrl+S")
            .clicked()
        {
            actions.push(Action::Save);
        }
        if editor.saved.is_some() && ui.button("Reload saved").clicked() {
            actions.push(Action::Reload);
        }
        if editor.is_dirty() {
            ui.label(
                RichText::new("Unsaved changes")
                    .size(12.0)
                    .color(Color32::from_rgb(160, 103, 24)),
            );
        }
    });
    ui.add_space(14.0);
    ui.label(RichText::new("Title").strong().color(INK));
    ui.add(
        egui::TextEdit::singleline(&mut editor.title)
            .font(egui::TextStyle::Heading)
            .hint_text("What did you work on?")
            .desired_width(f32::INFINITY),
    );
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Event date").strong().color(INK));
        ui.add(
            egui::TextEdit::singleline(&mut editor.event_date)
                .desired_width(112.0)
                .hint_text("YYYY-MM-DD"),
        );
    });
    ui.label(
        RichText::new("When the work happened; you can document past work.")
            .size(11.0)
            .color(MUTED),
    );
    ui.add_space(18.0);
    ui.label(RichText::new("Research notes").strong().color(INK));
    ui.label(
        RichText::new("Conditions, observations, interpretation, decisions, and next steps.")
            .size(11.0)
            .color(MUTED),
    );
    ui.add_space(8.0);
    let notes_height = (viewport_height - (ui.cursor().top() - content_top) - 100.0).max(140.0);
    egui::ScrollArea::vertical().id_salt("notes").max_height(notes_height).show(ui, |ui| {
        ui.add_sized([ui.available_width(), notes_height], egui::TextEdit::multiline(&mut editor.body)
            .hint_text("Write your notes here. Values and units are preserved exactly as you enter them.")
            .desired_width(f32::INFINITY));
    });
    ui.add_space(14.0);
    if let Some(saved) = &editor.saved {
        ui.label(
            RichText::new(format!(
                "Created {} UTC  ·  Edited {} UTC  ·  Revision {}",
                saved.created_at.format("%Y-%m-%d %H:%M"),
                saved.updated_at.format("%Y-%m-%d %H:%M"),
                saved.revision
            ))
            .size(10.0)
            .color(MUTED),
        );
        ui.add(
            egui::Label::new(
                RichText::new(format!("Entry ID: {}", saved.id))
                    .size(10.0)
                    .color(MUTED),
            )
            .truncate(),
        );
    }
}

pub(crate) fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(Color32::WHITE))
            .fill(TEAL)
            .min_size(Vec2::new(100.0, 34.0))
            .corner_radius(5.0),
    )
}

fn configure_style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Light);
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();
    style.visuals.override_text_color = Some(INK);
    style.visuals.selection.bg_fill = Color32::from_rgb(190, 223, 214);
    style.visuals.selection.stroke = Stroke::new(1.0, TEAL);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(244, 247, 245);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(151, 166, 175));
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_gray(214));
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, TEAL);
    style.visuals.text_edit_bg_color = Some(Color32::WHITE);
    style.visuals.weak_text_color = Some(MUTED);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(227, 239, 234);
    style.visuals.extreme_bg_color = Color32::from_rgb(252, 253, 251);
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    // Apply after system-theme selection so Windows theme events keep the same fields.
    ctx.set_style(style);
}
