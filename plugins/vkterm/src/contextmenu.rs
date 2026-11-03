//! Menú contextual del editor (clic derecho), como el de VS Code
//! (src/menu.rs): ↑ ↓ y Enter, o el ratón; Esc, un clic fuera o perder el
//! foco lo cierran.
//!
//! Es otra instancia del plugin (configuración `mode "menu"`), como la
//! paleta. El plugin Lua de Micro avisa del clic derecho al explorador
//! (controlador), que la abre ya colocada donde se hizo clic, con el mensaje
//! `menu` y "panel columna fila" de payload: el panel de Micro y la esquina
//! de la ventana. Lo elegido vuelve al controlador en un mensaje `palette`
//! (`menu_action`), que lo ejecuta en ese panel cuando la ventana ya se ha
//! cerrado.
use crate::floating::{self, Floating, FRAME};
use vkterm::menu::{self, Entry, Menu, PAD};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Menú";

#[derive(Default)]
pub struct ContextMenu {
    floating: Floating,
    controller: Option<u32>,
    menu: Menu,
    /// El panel de Micro del clic, que recibe la acción.
    pane: String,
}

impl ContextMenu {
    pub fn load(&mut self) {
        self.floating.load(&[]);
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "menu" {
            return false;
        }
        let payload = message.payload.unwrap_or_default();
        let fields: Vec<&str> = payload.split_whitespace().collect();
        let [pane, left, top] = fields[..] else {
            return false;
        };
        self.pane = pane.to_string();
        self.menu = Menu::new(menu::EDITOR);
        if !self.floating.open(TITLE) {
            // Otro clic derecho antes de que se cerrara: se mueve a su sitio.
            let (width, height) = self.menu.size();
            let coordinates = FloatingPaneCoordinates::new(
                Some(left.to_string()),
                Some(top.to_string()),
                Some(width.to_string()),
                Some(height.to_string()),
                None,
                Some(true),
            );
            if let Some(coordinates) = coordinates {
                change_floating_panes_coordinates(vec![(PaneId::Plugin(self.floating.me), coordinates)]);
            }
            focus_plugin_pane(self.floating.me, true, false);
        }
        true
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
            Event::PaneUpdate(manifest) => {
                self.controller = floating::controller(&manifest, self.floating.me);
                false
            }
            Event::Key(key) => {
                match key.bare_key {
                    BareKey::Esc => close_self(),
                    BareKey::Down => self.menu.down(),
                    BareKey::Up => self.menu.up(),
                    BareKey::Enter | BareKey::Char(' ') if key.has_no_modifiers() => self.choose(),
                    _ => return false,
                }
                true
            }
            // En macOS, el clic derecho sobre una entrada también la elige.
            Event::Mouse(Mouse::LeftClick(line, _) | Mouse::RightClick(line, _)) => {
                let Some(i) = row(line).and_then(|r| self.menu.at(r)) else {
                    return false;
                };
                self.menu.selected = Some(i);
                self.choose();
                true
            }
            Event::Mouse(Mouse::Hover(line, _)) => {
                let before = self.menu.selected;
                if let Some(r) = row(line) {
                    self.menu.hover(r);
                }
                self.menu.selected != before
            }
            _ => false,
        }
    }

    fn choose(&mut self) {
        let Some(action) = self.menu.action() else {
            return;
        };
        if let Some(controller) = self.controller {
            pipe_message_to_plugin(
                MessageToPlugin::new("palette")
                    .with_destination_plugin_id(controller)
                    .with_payload(format!("menu_action\t{} {action}", self.pane)),
            );
        }
        close_self();
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let palette = self.floating.palette;
        floating::frame(rows, cols, "", "", &palette);
        let width = cols.saturating_sub(2 * FRAME);
        let keys_width = self.menu.keys_width();
        let label_width = width.saturating_sub(2 * PAD + keys_width);
        let pad = " ".repeat(PAD);
        for (i, entry) in self.menu.entries.iter().enumerate() {
            let y = FRAME + i;
            if y + FRAME >= rows {
                break;
            }
            let line = match entry {
                Entry::Separator => {
                    let mut line = floating::row(&"─".repeat(width), width, &palette);
                    line.fg(0..width, palette.border);
                    line
                }
                Entry::Item { label, keys, .. } => {
                    let text = format!("{pad}{label:<label_width$}{keys:>keys_width$}{pad}");
                    let mut line = floating::row(&text, width, &palette);
                    if self.menu.selected == Some(i) {
                        line.paint(0..width, |_| palette.quick_selection);
                    } else {
                        line.fg(PAD + label_width..width, palette.keybinding);
                    }
                    line
                }
            };
            print!("{}", line.render(FRAME, y));
        }
    }
}

/// Fila del interior para una fila de la ventana (la 0 es el marco).
fn row(line: isize) -> Option<usize> {
    usize::try_from(line).ok()?.checked_sub(FRAME)
}
