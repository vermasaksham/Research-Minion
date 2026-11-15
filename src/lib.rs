//! Local storage and editing rules, independent of the desktop interface.
#![recursion_limit = "256"]
pub mod chemistry;
pub mod cvt;
pub mod editor;
pub mod markdown;
pub mod model;
pub mod note_images;
pub mod store;
pub mod thermo_database;
pub mod thermodynamics;
pub mod xlsx_template;

#[cfg(feature = "desktop")]
mod thermo_view;

#[cfg(feature = "desktop")]
mod thermo_calculator_view;

#[cfg(feature = "desktop")]
pub mod app;

#[cfg(feature = "desktop")]
mod cvt_view;

#[cfg(feature = "desktop")]
mod materials_view;

pub mod peak;
pub mod peak_reference;

#[cfg(feature = "desktop")]
mod peak_view;

pub mod profiles;

#[cfg(feature = "desktop")]
mod profile_view;

#[cfg(feature = "desktop")]
mod notes_view;

#[cfg(feature = "desktop")]
mod manual_peak_view;

#[cfg(feature = "desktop")]
mod peak_plot;
