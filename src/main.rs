#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::path::PathBuf;

use research_minion::{app::ResearchMinionApp, profiles::Profiles, store::default_data_dir};

fn main() -> eframe::Result {
    let data_dir = parse_data_dir().and_then(|path| {
        path.map(Ok)
            .unwrap_or_else(|| default_data_dir().map_err(|e| e.to_string()))
    });
    // Open errors are shown in the desktop window, not hidden in a Windows console.
    let store = data_dir.and_then(|path| {
        Profiles::open(&path).map_err(|e| format!("{}\n\nLocation: {}", e, path.display()))
    });
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Research Minion")
            .with_inner_size([1240.0, 820.0])
            .with_min_inner_size([1000.0, 650.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Research Minion",
        options,
        Box::new(move |cc| Ok(Box::new(ResearchMinionApp::new(cc, store)))),
    )
}

fn parse_data_dir() -> Result<Option<PathBuf>, String> {
    let mut args = std::env::args_os().skip(1);
    let mut result = None;
    while let Some(arg) = args.next() {
        if arg == "--data-dir" && result.is_none() {
            result = Some(PathBuf::from(
                args.next().ok_or("--data-dir needs a folder path.")?,
            ));
        } else {
            return Err("Usage: research-minion [--data-dir <folder>]".into());
        }
    }
    Ok(result)
}
