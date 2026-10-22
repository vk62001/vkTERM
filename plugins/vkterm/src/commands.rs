//! Comandos de la paleta (⌘⇧P), con los nombres de VS Code ("Ver: …",
//! "Archivo: …"). Cada uno es un mensaje al controlador (src/main.rs,
//! `dispatch`), salvo los que la propia paleta resuelve (`palette_*`).
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub label: &'static str,
    /// Atajo que se muestra a la derecha ("" si no tiene).
    pub keys: &'static str,
    pub message: &'static str,
    pub payload: &'static str,
}

const fn cmd(label: &'static str, keys: &'static str, message: &'static str, payload: &'static str) -> Command {
    Command { label, keys, message, payload }
}

/// Lo que dicen los atajos debe coincidir con src/shortcuts.rs.
pub const COMMANDS: &[Command] = &[
    cmd("Archivo: Abrir archivo…", "⌘P", "palette_files", ""),
    cmd("Archivo: Nuevo archivo", "", "editor_key", "new_tab"),
    cmd("Archivo: Guardar", "⌘S", "editor_key", "save"),
    cmd("Ver: Mostrar u ocultar el explorador", "⌘B", "toggle_sidebar", ""),
    cmd("Ver: Mostrar u ocultar el Shell", "⌘J", "toggle_panel", ""),
    cmd("Ver: Mostrar u ocultar Claude Code y Codex", "⌥⌘B", "toggle_agents", ""),
    cmd("Ver: Foco al explorador", "⌘1", "focus", "explorer"),
    cmd("Ver: Foco al editor", "⌘2", "focus", "editor"),
    cmd("Ver: Foco a Claude Code", "⌘3", "focus", "claude"),
    cmd("Ver: Foco a Codex", "⌘4", "focus", "codex"),
    cmd("Ver: Foco al Shell", "⌘5", "focus", "shell"),
    cmd("Ver: Ampliar o restaurar el panel", "⌘K Z", "zoom", ""),
    cmd("Ver: Mostrar u ocultar .git, .DS_Store… en el explorador", "Ctrl h", "toggle_excluded", ""),
    cmd("Editor: Pestaña siguiente", "⌃Tab", "next_tab", ""),
    cmd("Editor: Pestaña anterior", "⌃⇧Tab", "prev_tab", ""),
    cmd("Editor: Cerrar la pestaña", "⌘W", "editor_key", "close_tab"),
    cmd("Editor: Deshacer", "⌘Z", "editor_key", "undo"),
    cmd("Editor: Rehacer", "⌘⇧Z", "editor_key", "redo"),
    cmd("Editor: Seleccionar todo", "⌘A", "editor_key", "select_all"),
    cmd("Editor: Buscar en el archivo", "⌘F", "editor_key", "find"),
    cmd("Preferencias: Tema de color", "⌘K ⌘T", "open_window", "themes"),
    cmd("Ayuda: Atajos de teclado", "⌘K ⌘S", "open_window", "shortcuts"),
    cmd("Ayuda: Paleta de comandos", "⌘⇧P", "palette_commands", ""),
];

/// Teclas que el controlador escribe en Micro para `editor_key`.
pub fn editor_key(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "save" => b"\x1bOQ", // F2: Guardar (config/micro/bindings.json)
        "new_tab" => b"\x14",
        "close_tab" => b"\x11",
        "undo" => b"\x1a",
        "redo" => b"\x19",
        "select_all" => b"\x01",
        "find" => b"\x06",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn etiquetas_unicas_con_categoria() {
        let labels: HashSet<_> = COMMANDS.iter().map(|c| c.label).collect();
        assert_eq!(labels.len(), COMMANDS.len());
        assert!(COMMANDS.iter().all(|c| c.label.contains(": ")));
    }

    #[test]
    fn toda_tecla_de_editor_key_existe() {
        for c in COMMANDS.iter().filter(|c| c.message == "editor_key") {
            assert!(editor_key(c.payload).is_some(), "{}", c.payload);
        }
    }

    #[test]
    fn los_atajos_coinciden_con_la_ventana_de_atajos() {
        let shown: Vec<&str> = crate::shortcuts::CATEGORIES
            .iter()
            .flat_map(|c| c.entries.iter().map(|(k, _)| *k))
            .collect();
        for c in COMMANDS.iter().filter(|c| !c.keys.is_empty()) {
            let parts: Vec<&str> = c.keys.split_whitespace().collect();
            assert!(
                shown.iter().any(|k| parts.iter().all(|p| k.split_whitespace().any(|s| s == *p))),
                "{} no está en shortcuts.rs",
                c.keys
            );
        }
    }
}
