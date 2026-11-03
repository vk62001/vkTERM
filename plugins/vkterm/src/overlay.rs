//! Ventana de atajos (⌘K ⌘S), como el "Keyboard Shortcuts" de Unity: un panel
//! flotante centrado con la lista de src/shortcuts.rs. Con los colores del
//! editor del tema y las teclas en etiquetas, como el editor «Keyboard
//! Shortcuts» de VS Code (⌘K ⌘S); las demás ventanas son de Quick Input.
//!
//! Es otra instancia del mismo plugin (configuración `mode "shortcuts"`): el
//! atajo le manda `toggle_shortcuts` y Zellij la lanza si no existe. La
//! primera vez se muestra; la siguiente se cierra. También se cierra con
//! cualquier tecla, con un clic o al perder el foco. Mientras tiene el foco,
//! el controlador atenúa el resto de paneles, que hacen de fondo.
use crate::floating::{self, Floating, FRAME};
use vkterm::shortcuts::{self, Style};
use vkterm::style::Palette;
use zellij_tile::prelude::*;

const TITLE: &str = "Atajos de teclado";
/// Márgenes dentro del marco: columnas a cada lado y filas arriba y abajo.
const PAD_X: usize = 2;
const PAD_Y: usize = 1;

#[derive(Default)]
pub struct Overlay {
    floating: Floating,
    scroll: usize,
    max_scroll: usize,
}

impl Overlay {
    pub fn load(&mut self) {
        self.floating.load(&[]);
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "toggle_shortcuts" {
            return false;
        }
        if !self.floating.open(TITLE) {
            close_self();
        }
        false
    }

    pub fn update(&mut self, event: Event) -> bool {
        if self.floating.lost_focus(&event) {
            close_self();
            return false;
        }
        if self.floating.theme(&event) {
            return true;
        }
        match event {
            Event::PaneUpdate(_) => {
                // Del tamaño de su contenido para el ancho disponible.
                let (columns, _) = self.floating.area();
                let lines = shortcuts::layout(columns.saturating_sub(2 * (FRAME + PAD_X)));
                let content = lines.iter().map(|l| l.width()).max().unwrap_or(0);
                self.floating.place(content + 2 * (FRAME + PAD_X), lines.len() + 2 * (FRAME + PAD_Y));
                false
            }
            Event::Key(key) => {
                let step = match key.bare_key {
                    _ if !key.has_no_modifiers() || self.max_scroll == 0 => None,
                    BareKey::Down => Some(1),
                    BareKey::Up => Some(-1),
                    BareKey::PageDown => Some(10),
                    BareKey::PageUp => Some(-10),
                    _ => None,
                };
                match step {
                    Some(step) => self.scroll_by(step),
                    None => close_self(),
                }
                true
            }
            Event::Mouse(Mouse::ScrollDown(_)) => {
                self.scroll_by(1);
                true
            }
            Event::Mouse(Mouse::ScrollUp(_)) => {
                self.scroll_by(-1);
                true
            }
            Event::Mouse(Mouse::LeftClick(..)) => {
                close_self();
                false
            }
            _ => false,
        }
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        // El marco y las filas de las ventanas van sobre el fondo de Quick
        // Input (`quick`): aquí, sobre el del editor.
        let palette = Palette { quick: self.floating.palette.editor, ..self.floating.palette };
        let width = cols.saturating_sub(2 * (FRAME + PAD_X));
        let lines = shortcuts::layout(width);
        let height = rows.saturating_sub(2 * (FRAME + PAD_Y));
        self.max_scroll = lines.len().saturating_sub(height);
        self.scroll = self.scroll.min(self.max_scroll);
        let hint = if self.max_scroll > 0 {
            "↑ ↓ desplazar · otra tecla cierra"
        } else {
            "Esc o cualquier tecla para cerrar"
        };
        floating::frame(rows, cols, TITLE, hint, &palette);
        for (y, line) in lines.iter().skip(self.scroll).take(height).enumerate() {
            let mut row = floating::row(&line.text, width, &palette);
            for (style, range) in &line.spans {
                match style {
                    Style::Heading => row.paint(range.clone(), |l| l.with_fg(palette.accent).bold()),
                    Style::Key => row.paint(range.clone(), |_| palette.keycap),
                    Style::Rule => row.fg(range.clone(), palette.border),
                }
            }
            print!("{}", row.render(FRAME + PAD_X, FRAME + PAD_Y + y));
        }
    }

    fn scroll_by(&mut self, delta: isize) {
        self.scroll = self.scroll.saturating_add_signed(delta).min(self.max_scroll);
    }
}
