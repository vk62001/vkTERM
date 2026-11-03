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
    cmd("Archivo: Nuevo archivo…", "⌘N", "new_file", ""),
    cmd("Archivo: Nuevo archivo de texto sin título", "", "editor_key", "new_tab"),
    cmd("Archivo: Guardar", "⌘S", "editor_key", "save"),
    cmd("Archivo: Abrir reciente…", "⌘O", "open_window", "recent"),
    cmd("Archivo: Cerrar carpeta", "⌘K F", "close_folder", ""),
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
    cmd("Ver: Explorador", "⌘⇧E", "show_view", "explorer"),
    cmd("Ver: Control de código fuente", "⌃⇧G", "show_view", "scm"),
    cmd("Git: Commit", "⌘Enter", "git", "commit"),
    cmd("Git: Preparar todos los cambios", "", "git", "stage_all"),
    cmd("Git: Quitar todos los cambios preparados", "", "git", "unstage_all"),
    cmd("Git: Pull", "", "git", "pull"),
    cmd("Git: Push", "", "git", "push"),
    cmd("Git: Sincronizar (pull y push)", "", "git", "sync"),
    cmd("Git: Fetch", "", "git", "fetch"),
    cmd("Git: Comparar con otra rama (conflictos al fusionar)…", "", "git", "compare"),
    cmd("Git: Abortar la fusión", "", "git", "abort"),
    cmd("Git: Actualizar", "", "git", "refresh"),
    cmd("Editor: Pestaña siguiente", "⌃Tab", "next_tab", ""),
    cmd("Editor: Pestaña anterior", "⌃⇧Tab", "prev_tab", ""),
    cmd("Editor: Cerrar la pestaña", "⌘W", "editor_key", "close_tab"),
    cmd("Editor: Deshacer", "⌘Z", "editor_key", "undo"),
    cmd("Editor: Rehacer", "⌘⇧Z", "editor_key", "redo"),
    cmd("Editor: Seleccionar todo", "⌘A", "editor_key", "select_all"),
    cmd("Editor: Buscar en el archivo", "⌘F", "editor_key", "find"),
    cmd("Ver: Aumentar el tamaño de letra", "⌘+", "font_size", "+1"),
    cmd("Ver: Reducir el tamaño de letra", "⌘-", "font_size", "-1"),
    cmd("Ver: Restablecer el tamaño de letra", "⌘0", "font_size", "0"),
    cmd("Preferencias: Tema de color", "⌘K ⌘T", "open_window", "themes"),
    cmd("Preferencias: Fuente", "⌘K ⌘L", "open_window", "fonts"),
    cmd("Preferencias: Recargar la configuración de Ghostty", "⌘⇧,", "reload_config", ""),
    cmd("Ayuda: Atajos de teclado", "⌘K ⌘S", "open_window", "shortcuts"),
    cmd("Ayuda: Paleta de comandos", "⌘⇧P", "palette_commands", ""),
];

/// Teclas que el controlador escribe en Micro para `editor_key` y para el
/// menú contextual (src/menu.rs).
pub fn editor_key(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "save" => b"\x1bOQ", // F2: Guardar (config/micro/bindings.json)
        "new_tab" => b"\x14",
        "close_tab" => b"\x11",
        "undo" => b"\x1a",
        "redo" => b"\x19",
        "select_all" => b"\x01",
        "find" => b"\x06",
        "cut" => b"\x18",
        "copy" => b"\x03",
        "paste" => b"\x16",
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
