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
    cmd("Archivo: Renombrar…", "⌘R", "rename_file", ""),
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
    cmd("Ver: Copiar todo el historial del panel", "⌘K ⌘C", "copy_pane", ""),
    cmd("Ver: Ampliar o restaurar el panel", "⌘K Z", "zoom", ""),
    cmd("Ver: Mostrar u ocultar .git, .DS_Store… en el explorador", "Ctrl h", "toggle_excluded", ""),
    cmd("Ver: Menú del clic derecho del explorador", "⌘.", "explorer_menu", ""),
    cmd("Ver: Explorador", "⌘⇧E", "show_view", "explorer"),
    cmd("Ver: Buscar en los archivos", "⌘⇧F", "show_view", "search"),
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
    cmd("Editor: Añadir cursor en la siguiente coincidencia", "⌘D", "editor_key", "multicursor"),
    cmd("Editor: Comentar o descomentar la línea", "⌘/", "editor_key", "comment"),
    cmd("Editor: Indentar", "⌘]", "editor_key", "indent"),
    cmd("Editor: Quitar sangría", "⌘[", "editor_key", "outdent"),
    cmd("Editor: Siguiente coincidencia", "⌘G", "editor_key", "find_next"),
    cmd("Editor: Anterior coincidencia", "⌘⇧G", "editor_key", "find_prev"),
    cmd("Editor: Seleccionar la línea", "⌘L", "editor_key", "select_line"),
    cmd("Editor: Borrar la línea", "⌘⇧K", "editor_key", "delete_line"),
    cmd("Editor: Ir a la línea…", "", "editor_key", "goto_line"),
    cmd("Git: Historial del archivo", "", "editor_key", "history"),
    cmd("Git: Blame del archivo (quién cambió cada línea)", "", "editor_key", "blame"),
    cmd("Git: Iniciar repositorio", "", "git", "init"),
    cmd("Editor: Dividir a la derecha", "", "editor_key", "split_right"),
    cmd("Editor: Dividir abajo", "", "editor_key", "split_down"),
    cmd("Archivo: Activar el autoguardado", "", "editor_key", "autosave_on"),
    cmd("Archivo: Desactivar el autoguardado", "", "editor_key", "autosave_off"),
    cmd("Lenguaje: Ir a la definición", "F12", "editor_key", "lsp_definition"),
    cmd("Lenguaje: Ir a la declaración", "", "editor_key", "lsp_declaration"),
    cmd("Lenguaje: Ir a la implementación", "", "editor_key", "lsp_implementation"),
    cmd("Lenguaje: Buscar todas las referencias", "⇧F12", "editor_key", "lsp_references"),
    cmd("Lenguaje: Mostrar información", "⌘K ⌘I", "editor_key", "lsp_hover"),
    cmd("Lenguaje: Sugerencias", "⌘I", "editor_key", "lsp_complete"),
    cmd("Lenguaje: Cambiar el nombre del símbolo", "⌘K ⌘R", "editor_key", "lsp_rename"),
    cmd("Lenguaje: Dar formato al documento", "⌘K ⌘F", "editor_key", "lsp_format"),
    cmd("Lenguaje: Símbolos del archivo", "⌘⇧O", "editor_key", "lsp_symbols"),
    cmd("Lenguaje: Detalle del error de la línea", "", "editor_key", "lsp_diagnostic"),
    cmd("Lenguaje: Iniciar el servidor", "", "editor_key", "lsp_start"),
    cmd("Lenguaje: Detener los servidores", "", "editor_key", "lsp_stop"),
    cmd("Lenguaje: Ver el registro", "", "editor_key", "lsp_log"),
    cmd("Editor: Reemplazar en el archivo", "⌥⌘F", "editor_key", "replace"),
    cmd("Ver: Aumentar el tamaño de letra", "⌘+", "font_size", "+1"),
    cmd("Ver: Reducir el tamaño de letra", "⌘-", "font_size", "-1"),
    cmd("Ver: Restablecer el tamaño de letra", "⌘0", "font_size", "0"),
    cmd("Preferencias: Tema de color", "⌘K ⌘T", "open_window", "themes"),
    cmd("Preferencias: Fuente", "⌘K ⌘L", "open_window", "fonts"),
    cmd("Preferencias: Recargar la configuración de Ghostty", "⌘⇧,", "reload_config", ""),
    cmd("Ayuda: Atajos de teclado", "⌘K ⌘S", "open_window", "shortcuts"),
    cmd("Ayuda: Paleta de comandos", "⌘⇧P", "palette_commands", ""),
];

/// Teclas que el controlador escribe en Micro para los atajos ⌘ de edición
/// (mensaje `edit`). Las `Alt-<carácter>` están ligadas a su acción en
/// config/micro/bindings.json.
pub fn edit_key(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "undo" => b"\x1a",
        "redo" => b"\x19",
        "select_all" => b"\x01",
        "find" => b"\x06",
        "cut" => b"\x18",
        "copy" => b"\x03",
        "multicursor" => b"\x1bn", // SpawnMultiCursor (por defecto de Micro)
        "comment" => b"\x1f",      // Ctrl-/ : plugin comment de Micro
        "indent" => b"\x1b$",
        "outdent" => b"\x1b%",
        "find_next" => b"\x1b&",
        "find_prev" => b"\x1b(",
        "select_line" => b"\x1b)",
        "delete_line" => b"\x1b<",
        // Lenguaje: comandos de µlsp en la línea de comandos de Micro (Ctrl-E).
        "lsp_complete" => b"\x05lsp autocomplete\r",
        "lsp_hover" => b"\x05lsp hover\r",
        "lsp_references" => b"\x05lsp find-references\r",
        "lsp_format" => b"\x05lsp format\r",
        "lsp_rename" => b"\x05lsp rename\r",
        "lsp_symbols" => b"\x05lsp document-symbols\r",
        "lsp_definition" => b"\x05lsp goto-definition\r",
        "lsp_declaration" => b"\x05lsp goto-declaration\r",
        "lsp_implementation" => b"\x05lsp goto-implementation\r",
        "lsp_diagnostic" => b"\x05lsp diagnostic-info\r",
        "lsp_log" => b"\x05lsp showlog\r",
        // Plugin Lua vkterm de Micro (config/micro/plugins/vkterm) y ajustes.
        "history" => b"\x05vkterm.history\r",
        "blame" => b"\x05vkterm.blame\r",
        "split_right" => b"\x05vkterm.splitright\r",
        "split_down" => b"\x05vkterm.splitdown\r",
        "autosave_on" => b"\x05set autosave 2\r",
        "autosave_off" => b"\x05set autosave 0\r",
        "lsp_start" => b"\x05lsp start\r",
        "lsp_stop" => b"\x05lsp stop\r",
        "replace" => b"\x05vkterm.replace\r",
        _ => return None,
    })
}

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
        // Ctrl-E (línea de comandos de Micro) y vkterm.reveal, del plugin Lua.
        "reveal" => b"\x05vkterm.reveal\r",
        // Ctrl-E y «goto »: el número lo escribe el usuario.
        "goto_line" => b"\x05goto ",
        other => return edit_key(other),
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
