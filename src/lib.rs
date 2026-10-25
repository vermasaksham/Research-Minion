//! Local storage and editing rules, independent of the desktop interface.
pub mod cvt;
pub mod editor;
pub mod model;
pub mod store;

#[cfg(feature = "desktop")]
pub mod app;

#[cfg(feature = "desktop")]
mod cvt_view;
