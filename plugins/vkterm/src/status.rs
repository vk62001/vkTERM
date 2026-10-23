//! Barra de estado, como la de VS Code: abajo, de lado a lado. A la izquierda
//! la carpeta, la rama de git y el modo de Zellij cuando no es el bloqueado
//! (tras Ctrl g); a la derecha, cómo cerrar la carpeta y ver los atajos. El
//! centro queda libre: ahí dibuja Ghostty su indicador del modo IDE
//! («⌨ vkterm»), que no se puede ocultar.
//!
//! Es otra instancia del plugin (configuración `mode "status"`) y no pide
//! permisos: en un panel de una fila no se podría responder a la pregunta. El
//! explorador (controlador) le manda la rama y el modo en un mensaje `status`.
use zellij_tile::prelude::*;

const HINTS: &str = "⌘K F cerrar carpeta  ·  ⌘K ⌘S atajos ";

#[derive(Default)]
pub struct StatusBar {
    project: String,
    branch: String,
    mode: String,
}

impl StatusBar {
    pub fn load(&mut self) {
        set_selectable(false);
        self.project = get_plugin_ids()
            .initial_cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
    }

    /// payload: "<rama>\t<modo>" (vacíos si no hay rama o el modo es LOCKED).
    /// Siempre se vuelve a dibujar: el explorador también lo manda cuando
    /// cambia el tema de Zellij.
    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "status" {
            return false;
        }
        let payload = message.payload.unwrap_or_default();
        let (branch, mode) = payload.split_once('\t').unwrap_or((&payload, ""));
        self.branch = branch.to_string();
        self.mode = mode.to_string();
        true
    }

    pub fn render(&mut self, _rows: usize, cols: usize) {
        let mut left = format!(" {}", self.project);
        let project_end = left.chars().count();
        if !self.branch.is_empty() {
            left += &format!("   ⎇ {}", self.branch);
        }
        let mode_start = left.chars().count() + 3;
        if !self.mode.is_empty() {
            left += &format!("   {} ", self.mode);
        }
        let used = left.chars().count();
        let hints = if used + HINTS.chars().count() + 4 <= cols { HINTS } else { "" };
        let gap = cols.saturating_sub(used + hints.chars().count());
        let line = crate::fit(&format!("{left}{}{hints}", " ".repeat(gap)), cols);
        let mut text = Text::new(&line).selected().color_range(2, 1..project_end);
        if !self.mode.is_empty() {
            text = text.color_range(3, mode_start..mode_start + self.mode.chars().count());
        }
        print_text_with_coordinates(text, 0, 0, Some(cols), Some(1));
    }
}
