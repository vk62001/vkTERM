//! Barra de estado, como la de VS Code: abajo, de lado a lado. A la izquierda
//! el distintivo del modo IDE, la carpeta, la rama de git (con "*" si hay
//! cambios y los commits por bajar y por subir), los conflictos (de una
//! fusión en curso o los que habría al fusionar la rama principal) y el modo
//! de Zellij cuando no es el bloqueado (tras Ctrl g); a la derecha, cómo
//! cerrar la carpeta y ver los atajos. El distintivo sustituye al indicador de Ghostty
//! («⌨ vkterm» flotando sobre el terminal), que vkTERM.app ya no dibuja
//! (patches/ghostty).
//!
//! Es otra instancia del plugin (configuración `mode "status"`) y no pide
//! permisos: en un panel de una fila no se podría responder a la pregunta.
//! Sin permisos tampoco recibe ModeUpdate: el explorador (controlador) le
//! manda la rama, el modo y los colores del tema (statusBar.* de VS Code) en
//! un mensaje `status`.
use vkterm::style::{Line, Look, Palette, Rgb};
use zellij_tile::prelude::*;

const HINTS: &str = "⌘K F cerrar carpeta  ·  ⌘K ⌘S atajos ";
/// El modo IDE (la tabla de atajos `vkterm` de Ghostty), como el distintivo
/// de la izquierda de VS Code.
const BADGE: &str = " ⌨ vkTERM ";

pub struct StatusBar {
    project: String,
    branch: String,
    notice: String,
    mode: String,
    look: Look,
    item: Look,
}

impl Default for StatusBar {
    fn default() -> StatusBar {
        let palette = Palette::default();
        StatusBar {
            project: String::new(),
            branch: String::new(),
            notice: String::new(),
            mode: String::new(),
            look: palette.status,
            item: palette.status_item,
        }
    }
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

    /// payload: "<rama>\t<modo>\t<colores>\t<aviso>" (rama y modo vacíos si
    /// no hay rama o el modo es LOCKED; colores: texto, fondo y los del
    /// distintivo, "#rrggbb" separados por comas, ver `colors`; aviso: los
    /// conflictos, o vacío).
    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "status" {
            return false;
        }
        let payload = message.payload.unwrap_or_default();
        let mut fields = payload.split('\t');
        self.branch = fields.next().unwrap_or_default().to_string();
        self.mode = fields.next().unwrap_or_default().to_string();
        let colors: Vec<Rgb> = fields.next().unwrap_or_default().split(',').filter_map(Rgb::parse).collect();
        self.notice = fields.next().unwrap_or_default().to_string();
        if let [fg, bg, item_fg, item_bg] = colors[..] {
            self.look = Look::new(fg, bg);
            self.item = Look::new(item_fg, item_bg);
        }
        true
    }

    pub fn render(&mut self, _rows: usize, cols: usize) {
        let badge_end = BADGE.chars().count();
        let mut left = format!("{BADGE} {}", self.project);
        let project_end = left.chars().count();
        if !self.branch.is_empty() {
            left += &format!("   ⎇ {}", self.branch);
        }
        let notice_start = left.chars().count() + 3;
        if !self.notice.is_empty() {
            left += &format!("   {}", self.notice);
        }
        let mode_start = left.chars().count() + 3;
        if !self.mode.is_empty() {
            left += &format!("   {} ", self.mode);
        }
        let used = left.chars().count();
        let hints = if used + HINTS.chars().count() + 4 <= cols { HINTS } else { "" };
        let gap = cols.saturating_sub(used + hints.chars().count());
        let mut line = Line::new(&format!("{left}{}{hints}", " ".repeat(gap)), cols, self.look);
        line.paint(0..badge_end, |_| self.item.bold());
        line.paint(badge_end + 1..project_end, |l| l.bold());
        line.paint(notice_start..notice_start + self.notice.chars().count(), |l| l.bold());
        if !self.mode.is_empty() {
            // El modo de Zellij, como el distintivo de la izquierda de VS Code.
            line.paint(mode_start - 1..mode_start + self.mode.chars().count() + 1, |_| self.item.bold());
        }
        print!("{}", line.render(0, 0));
    }
}

/// Los colores de la barra de estado para el mensaje `status`.
pub fn colors(palette: &Palette) -> String {
    [palette.status.fg, palette.status.bg, palette.status_item.fg, palette.status_item.bg]
        .map(Rgb::hex)
        .join(",")
}
