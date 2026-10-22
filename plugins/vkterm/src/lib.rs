//! Lógica del plugin vkterm que no depende de Zellij (se prueba con
//! `cargo test --lib --target aarch64-apple-darwin`).
pub mod clipboard;
pub mod commands;
pub mod fuzzy;
pub mod layout;
pub mod shortcuts;
pub mod themes;
pub mod tree;
