//! Colores de la interfaz del plugin, los del tema de VS Code elegido.
//!
//! scripts/gen-themes.py pone los colores de la interfaz de cada tema en los
//! componentes del tema de Zellij (config/zellij/themes/vkterm.kdl), y Zellij
//! se los manda a cada instancia del plugin en ModeUpdate (también al cambiar
//! de tema). Aquí se convierten en una paleta por función:
//!
//! | componente de Zellij | uso en vkTERM (color de VS Code) |
//! |---|---|
//! | text_unselected | barra lateral (sideBar.*); énfasis: coincidencias, descripción, foco y distintivo |
//! | text_selected | selección con foco (list.activeSelection*) |
//! | table_title | cabecera del explorador (sideBarSectionHeader.*) |
//! | table_cell_selected | selección sin foco (list.inactiveSelection*) |
//! | table_cell_unselected | ventana de atajos (editor.*); énfasis: las teclas (keybindingLabel.*) |
//! | list_unselected / list_selected | paleta y selectores (quickInput*, quickInputList.focus*); énfasis: coincidencias, atajos, descripción, grupos |
//! | ribbon_selected | barra de estado (statusBar.*); énfasis: el distintivo (statusBarItem.remote*) |
//! | frame_selected / frame_unselected | borde del panel con foco (focusBorder) / de los demás (editorGroup.border) |
//!
//! Se pinta con SGR de 24 bits, carácter a carácter (`Line`): sin negrita
//! obligatoria y con el fondo de cada zona de lado a lado.
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.
use std::fmt::Write;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// "#rrggbb" (mayúsculas o minúsculas).
    pub fn parse(hex: &str) -> Option<Rgb> {
        let hex = hex.trim().strip_prefix('#').filter(|h| h.len() == 6)?;
        let n = u32::from_str_radix(hex, 16).ok()?;
        Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
    }

    /// "#rrggbb", el formato con el que Zellij informa los colores.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    /// Un color de la paleta de 256 de xterm (temas de Zellij en ese formato).
    pub fn from_ansi256(n: u8) -> Rgb {
        const BASE: [Rgb; 16] = [
            Rgb(0, 0, 0), Rgb(205, 0, 0), Rgb(0, 205, 0), Rgb(205, 205, 0),
            Rgb(0, 0, 238), Rgb(205, 0, 205), Rgb(0, 205, 205), Rgb(229, 229, 229),
            Rgb(127, 127, 127), Rgb(255, 0, 0), Rgb(0, 255, 0), Rgb(255, 255, 0),
            Rgb(92, 92, 255), Rgb(255, 0, 255), Rgb(0, 255, 255), Rgb(255, 255, 255),
        ];
        match n {
            0..=15 => BASE[n as usize],
            16..=231 => {
                let level = |c: u8| if c == 0 { 0 } else { 55 + c * 40 };
                let i = n - 16;
                Rgb(level(i / 36), level(i / 6 % 6), level(i % 6))
            }
            _ => {
                let v = 8 + (n - 232) * 10;
                Rgb(v, v, v)
            }
        }
    }

    fn sgr(self) -> String {
        format!("{};{};{}", self.0, self.1, self.2)
    }
}

/// Texto y fondo de una celda.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Look {
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
}

impl Look {
    pub fn new(fg: Rgb, bg: Rgb) -> Look {
        Look { fg, bg, bold: false }
    }

    pub fn with_fg(self, fg: Rgb) -> Look {
        Look { fg, ..self }
    }

    pub fn bold(self) -> Look {
        Look { bold: true, ..self }
    }

    fn sgr(self) -> String {
        let weight = if self.bold { "1" } else { "22" };
        format!("\u{1b}[0;{weight};38;2;{};48;2;{}m", self.fg.sgr(), self.bg.sgr())
    }
}

/// Un componente del tema de Zellij: texto, fondo (si tiene) y cuatro
/// colores de énfasis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slot {
    pub base: Rgb,
    pub background: Option<Rgb>,
    pub emphasis: [Rgb; 4],
}

impl Slot {
    fn look(&self, fallback_bg: Rgb) -> Look {
        Look::new(self.base, self.background.unwrap_or(fallback_bg))
    }
}

/// Los componentes del tema de Zellij que usa vkTERM.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slots {
    pub text_unselected: Slot,
    pub text_selected: Slot,
    pub table_title: Slot,
    pub table_cell_selected: Slot,
    pub table_cell_unselected: Slot,
    pub list_unselected: Slot,
    pub list_selected: Slot,
    pub ribbon_selected: Slot,
    pub frame_selected: Slot,
    pub frame_unselected: Option<Slot>,
    pub exit_code_error: Slot,
}

/// La paleta de la interfaz, por función.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Barra lateral (explorador).
    pub sidebar: Look,
    /// Cabecera del explorador (nombre del proyecto).
    pub header: Look,
    /// Elemento elegido de una lista con el foco.
    pub selection: Look,
    /// Elemento elegido de una lista sin el foco.
    pub inactive_selection: Look,
    /// Texto secundario (ayudas, descripciones).
    pub description: Rgb,
    /// Color de acento: el del borde del foco (títulos, marca del actual).
    pub accent: Rgb,
    /// Paleta y selectores (Quick Input).
    pub quick: Look,
    pub quick_selection: Look,
    /// Caracteres que coinciden con lo escrito, sin y con selección.
    pub quick_highlight: Rgb,
    pub quick_selection_highlight: Rgb,
    pub keybinding: Rgb,
    pub quick_description: Rgb,
    /// Nombre de los grupos de una lista (pickerGroup.foreground).
    pub group: Rgb,
    /// Barra de estado y su distintivo de la izquierda.
    pub status: Look,
    pub status_item: Look,
    /// El editor (editor.foreground y editor.background) y, sobre él, las
    /// teclas de un atajo en su etiqueta (keybindingLabel.*).
    pub editor: Look,
    pub keycap: Look,
    /// Bordes de las ventanas (y de los paneles sin foco).
    pub border: Rgb,
    pub error: Rgb,
}

impl Palette {
    pub fn from_slots(s: &Slots) -> Palette {
        let sidebar = s.text_unselected.look(Rgb::default());
        let quick = s.list_unselected.look(sidebar.bg);
        let status = s.ribbon_selected.look(sidebar.bg);
        let editor = s.table_cell_unselected.look(sidebar.bg);
        Palette {
            sidebar,
            header: s.table_title.look(sidebar.bg),
            selection: s.text_selected.look(sidebar.bg),
            inactive_selection: s.table_cell_selected.look(sidebar.bg),
            description: s.text_unselected.emphasis[1],
            accent: s.text_unselected.emphasis[2],
            quick,
            quick_selection: s.list_selected.look(quick.bg),
            quick_highlight: s.list_unselected.emphasis[0],
            quick_selection_highlight: s.list_selected.emphasis[0],
            keybinding: s.list_unselected.emphasis[1],
            quick_description: s.list_unselected.emphasis[2],
            group: s.list_unselected.emphasis[3],
            status,
            status_item: Look::new(s.ribbon_selected.emphasis[0], s.ribbon_selected.emphasis[1]),
            editor,
            keycap: Look::new(s.table_cell_unselected.emphasis[0], s.table_cell_unselected.emphasis[1]),
            border: s.frame_unselected.map(|f| f.base).unwrap_or(s.text_unselected.emphasis[1]),
            error: s.exit_code_error.base,
        }
    }
}

impl Default for Palette {
    /// Hasta el primer ModeUpdate: grises neutros sobre fondo oscuro.
    fn default() -> Palette {
        let fg = Rgb(204, 204, 204);
        let bg = Rgb(30, 30, 30);
        let base = Look::new(fg, bg);
        let selection = Look::new(Rgb(255, 255, 255), Rgb(4, 57, 94));
        Palette {
            sidebar: base,
            header: base,
            selection,
            inactive_selection: Look::new(fg, Rgb(55, 55, 61)),
            description: Rgb(150, 150, 150),
            accent: Rgb(0, 127, 212),
            quick: base,
            quick_selection: selection,
            quick_highlight: Rgb(42, 170, 255),
            quick_selection_highlight: Rgb(42, 170, 255),
            keybinding: fg,
            quick_description: Rgb(150, 150, 150),
            group: Rgb(55, 148, 255),
            status: Look::new(Rgb(255, 255, 255), Rgb(0, 122, 204)),
            status_item: Look::new(Rgb(255, 255, 255), Rgb(0, 122, 204)),
            editor: Look::new(Rgb(212, 212, 212), bg),
            keycap: Look::new(fg, Rgb(47, 47, 47)),
            border: Rgb(68, 68, 68),
            error: Rgb(244, 135, 113),
        }
    }
}

/// Una fila de texto con un estilo por carácter. Todos los caracteres que se
/// usan ocupan una columna.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    chars: Vec<char>,
    looks: Vec<Look>,
}

impl Line {
    /// `text` en `width` columnas (recortado con "…" o relleno con espacios).
    pub fn new(text: &str, width: usize, look: Look) -> Line {
        let chars: Vec<char> = fit(text, width).chars().collect();
        let looks = vec![look; chars.len()];
        Line { chars, looks }
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Cambia el estilo de un tramo (posiciones de carácter; se recorta).
    pub fn paint(&mut self, range: Range<usize>, f: impl Fn(Look) -> Look) {
        let end = range.end.min(self.looks.len());
        for look in &mut self.looks[range.start.min(end)..end] {
            *look = f(*look);
        }
    }

    pub fn fg(&mut self, range: Range<usize>, fg: Rgb) {
        self.paint(range, |l| l.with_fg(fg));
    }

    /// Resalta caracteres sueltos (las coincidencias de la búsqueda).
    pub fn highlight(&mut self, indices: &[usize], fg: Rgb) {
        for &i in indices {
            self.paint(i..i + 1, |l| l.with_fg(fg).bold());
        }
    }

    /// La fila en la posición (x, y) del panel, como secuencias de escape.
    pub fn render(&self, x: usize, y: usize) -> String {
        let mut out = format!("\u{1b}[{};{}H", y + 1, x + 1);
        let mut current: Option<Look> = None;
        for (c, look) in self.chars.iter().zip(&self.looks) {
            if current != Some(*look) {
                out.push_str(&look.sgr());
                current = Some(*look);
            }
            out.push(*c);
        }
        let _ = write!(out, "\u{1b}[0m");
        out
    }
}

/// Recorta (con "…") o rellena con espacios hasta `width` columnas.
pub fn fit(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count > width {
        let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
        if width > 0 {
            cut.push('…');
        }
        cut
    } else {
        format!("{}{}", text, " ".repeat(width - count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colores_en_hex_y_en_256() {
        assert_eq!(Rgb::parse("#1E1E1E"), Some(Rgb(30, 30, 30)));
        assert_eq!(Rgb::parse("1e1e1e"), None);
        assert_eq!(Rgb(0, 122, 204).hex(), "#007acc");
        assert_eq!(Rgb::from_ansi256(196), Rgb(255, 0, 0));
        assert_eq!(Rgb::from_ansi256(244), Rgb(128, 128, 128));
    }

    #[test]
    fn la_paleta_sale_de_los_componentes() {
        let slot = |base: Rgb, bg: Option<Rgb>, e: [Rgb; 4]| Slot { base, background: bg, emphasis: e };
        let a = Rgb(1, 1, 1);
        let sidebar_bg = Rgb(37, 37, 38);
        let mut slots = Slots {
            text_unselected: slot(Rgb(204, 204, 204), Some(sidebar_bg), [Rgb(42, 170, 255), Rgb(150, 150, 150), Rgb(0, 127, 212), a]),
            list_unselected: slot(Rgb(204, 204, 204), None, [Rgb(9, 9, 9), Rgb(8, 8, 8), Rgb(7, 7, 7), Rgb(6, 6, 6)]),
            ribbon_selected: slot(Rgb(255, 255, 255), Some(Rgb(0, 122, 204)), [Rgb(255, 255, 255), Rgb(22, 130, 92), a, a]),
            table_cell_unselected: slot(Rgb(212, 212, 212), Some(Rgb(30, 30, 30)), [Rgb(204, 204, 204), Rgb(47, 47, 47), a, a]),
            ..Slots::default()
        };
        slots.frame_unselected = Some(slot(Rgb(68, 68, 68), None, [a; 4]));
        let p = Palette::from_slots(&slots);
        assert_eq!(p.sidebar, Look::new(Rgb(204, 204, 204), sidebar_bg));
        // Sin fondo propio, la paleta usa el de la barra lateral.
        assert_eq!(p.quick.bg, sidebar_bg);
        assert_eq!(p.status.bg, Rgb(0, 122, 204));
        assert_eq!(p.status_item, Look::new(Rgb(255, 255, 255), Rgb(22, 130, 92)));
        assert_eq!(p.editor, Look::new(Rgb(212, 212, 212), Rgb(30, 30, 30)));
        assert_eq!(p.keycap, Look::new(Rgb(204, 204, 204), Rgb(47, 47, 47)));
        assert_eq!((p.quick_highlight, p.keybinding, p.quick_description, p.group), (Rgb(9, 9, 9), Rgb(8, 8, 8), Rgb(7, 7, 7), Rgb(6, 6, 6)));
        assert_eq!(p.accent, Rgb(0, 127, 212));
        assert_eq!(p.border, Rgb(68, 68, 68));
    }

    #[test]
    fn una_fila_con_tramos_de_colores() {
        let base = Look::new(Rgb(1, 2, 3), Rgb(4, 5, 6));
        let mut line = Line::new("abc", 5, base);
        assert_eq!(line.len(), 5);
        line.fg(1..2, Rgb(9, 9, 9));
        let out = line.render(2, 0);
        assert_eq!(
            out,
            "\u{1b}[1;3H\u{1b}[0;22;38;2;1;2;3;48;2;4;5;6ma\u{1b}[0;22;38;2;9;9;9;48;2;4;5;6mb\u{1b}[0;22;38;2;1;2;3;48;2;4;5;6mc  \u{1b}[0m"
        );
    }

    #[test]
    fn recorta_con_puntos_suspensivos() {
        assert_eq!(fit("abcdef", 4), "abc…");
        assert_eq!(fit("ab", 4), "ab  ");
        assert_eq!(fit("ab", 0), "");
        let mut line = Line::new("ab", 2, Look::default());
        line.highlight(&[1, 7], Rgb(1, 1, 1));
        assert!(line.render(0, 0).contains("\u{1b}[0;1;38;2;1;1;1"));
    }
}
