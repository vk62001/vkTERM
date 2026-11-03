//! Lógica del plugin vkterm que no depende de Zellij (se prueba con
//! `cargo test --lib --target aarch64-apple-darwin`).
pub mod applier;
pub mod clipboard;
pub mod closing;
pub mod commands;
pub mod folders;
pub mod fuzzy;
pub mod git;
pub mod layout;
pub mod memory;
pub mod menu;
pub mod newfile;
pub mod question;
pub mod quickpick;
pub mod shortcuts;
pub mod style;
pub mod themes;
mod themes_data;
pub mod tree;
