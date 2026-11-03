//! Selector de tema (⌘K ⌘T), como "Preferences: Color Theme" de VS Code: los
//! temas agrupados (claros, oscuros y alto contraste), lo escrito filtra y al
//! moverse por la lista se prueba cada uno; Enter lo deja y Esc vuelve al que
//! había (también al cerrarlo con el atajo, un clic fuera o al perder el foco).
//!
//! Es otra instancia del plugin (configuración `mode "themes"`), como la
//! ventana de atajos. Solo dibuja: el tema lo aplica el explorador
//! (controlador), que recibe cada cambio en un mensaje `set_theme`. Así, un
//! cambio que espera turno (src/applier.rs) no se pierde al cerrar la ventana.
use std::collections::BTreeMap;

use crate::floating::{self, Floating, FRAME};
use vkterm::quickpick::{QuickPick, Row};
use vkterm::themes::{self, THEMES};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Tema de color";
const HINT: &str = "↑ ↓ probar · Enter aplicar · Esc cancelar";
const PLACEHOLDER: &str = "Escribe para filtrar los temas";
const PAD_X: usize = 2;
const LIST_ROWS: usize = 16;

pub struct Picker {
    floating: Floating,
    /// Explorador (controlador), que aplica el tema.
    controller: Option<u32>,
    /// Tema guardado al abrir: al que se vuelve si no se confirma.
    saved: usize,
    /// Tema aplicado ahora mismo (el guardado o el que se está probando).
    applied: usize,
    /// Los temas en el orden del selector (índices de THEMES).
    order: Vec<usize>,
    list: QuickPick,
}

impl Default for Picker {
    fn default() -> Picker {
        let order = themes::picker_order();
        let items = order.iter().map(|&i| (THEMES[i].name.to_string(), Some(THEMES[i].kind.group()))).collect();
        let saved = themes::index(themes::DEFAULT);
        Picker { floating: Floating::default(), controller: None, saved, applied: saved, order, list: QuickPick::new(items) }
    }
}

impl Picker {
    pub fn load(&mut self) {
        self.floating.load(&[EventType::RunCommandResult]);
        self.select_theme(self.saved);
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "toggle_themes" {
            return false;
        }
        if self.floating.open(TITLE) {
            // Hasta conocer el guardado se supone el predeterminado.
            run_command(&["vkterm-theme", "current"], context("current"));
        } else {
            self.cancel();
        }
        false
    }

    pub fn update(&mut self, event: Event) -> bool {
        if self.floating.lost_focus(&event) {
            self.cancel();
            return false;
        }
        if self.floating.theme(&event) {
            return true;
        }
        match event {
            Event::RunCommandResult(_, stdout, _, ctx) if ctx.get("vkterm").map(String::as_str) == Some("current") => {
                self.saved = themes::index(&String::from_utf8_lossy(&stdout));
                self.applied = self.saved;
                self.select_theme(self.saved);
                true
            }
            Event::PaneUpdate(manifest) => {
                self.controller = floating::controller(&manifest, self.floating.me);
                let names = THEMES.iter().map(|t| t.name.chars().count()).max().unwrap_or(0);
                let width = (names + 30 + 2 * (FRAME + PAD_X)).max(HINT.chars().count() + 4);
                let rows = (self.list.rows().len()).min(LIST_ROWS);
                self.floating.place(width, rows + 1 + 2 * FRAME);
                false
            }
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(Mouse::ScrollDown(_)) => {
                self.move_by(1);
                true
            }
            Event::Mouse(Mouse::ScrollUp(_)) => {
                self.move_by(-1);
                true
            }
            Event::Mouse(Mouse::LeftClick(line, _)) => {
                let row = line - (FRAME + 1) as isize;
                if row >= 0 && self.list.select_row(self.list.scroll() + row as usize) {
                    self.preview();
                    close_self();
                }
                true
            }
            _ => false,
        }
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        let plain = key.has_no_modifiers();
        let typed = plain || key.has_only_modifiers(&[KeyModifier::Shift]);
        match key.bare_key {
            BareKey::Down if plain => self.move_by(1),
            BareKey::Up if plain => self.move_by(-1),
            BareKey::PageDown if plain => self.move_by(LIST_ROWS as isize),
            BareKey::PageUp if plain => self.move_by(-(LIST_ROWS as isize)),
            BareKey::Home if plain => {
                self.list.first();
                self.preview();
            }
            BareKey::End if plain => {
                self.list.last();
                self.preview();
            }
            BareKey::Enter if plain => {
                if self.list.selected().is_some() {
                    close_self();
                }
            }
            BareKey::Esc => self.cancel(),
            BareKey::Backspace => {
                let mut query = self.list.query().to_string();
                query.pop();
                self.filter(&query);
            }
            BareKey::Char(c) if typed && !c.is_control() => {
                let query = format!("{}{c}", self.list.query());
                self.filter(&query);
            }
            _ => return false,
        }
        true
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let palette = self.floating.palette;
        floating::frame(rows, cols, TITLE, HINT, &palette);
        let x = FRAME + PAD_X;
        let width = cols.saturating_sub(2 * x);
        if width < 8 || rows < 2 * FRAME + 2 {
            return;
        }
        print!("{}", floating::input_line("›", self.list.query(), PLACEHOLDER, width, &palette).render(x, FRAME));
        self.list.set_height(rows.saturating_sub(2 * FRAME + 1));
        let selected = self.list.selected_row();
        let scroll = self.list.scroll();
        for (y, row) in self.list.visible().iter().enumerate() {
            let line = match row {
                Row::Group(title) => floating::group_line(title, width, &palette),
                Row::Item { index, matched } => {
                    let theme = self.order[*index];
                    let mark = if theme == self.saved { "● " } else { "  " };
                    let mut line = floating::row(&format!("{mark}{}", THEMES[theme].name), width, &palette);
                    let current = selected == Some(scroll + y);
                    if current {
                        line.paint(0..width, |_| palette.quick_selection);
                    }
                    line.fg(0..1, palette.accent);
                    let shifted: Vec<usize> = matched.iter().map(|i| i + 2).collect();
                    let highlight = if current { palette.quick_selection_highlight } else { palette.quick_highlight };
                    line.highlight(&shifted, highlight);
                    line
                }
            };
            print!("{}", line.render(x, FRAME + 1 + y));
        }
        if self.list.rows().is_empty() {
            let mut line = floating::row("No hay temas que coincidan", width, &palette);
            line.fg(0..width, palette.quick_description);
            print!("{}", line.render(x, FRAME + 1));
        }
    }

    fn select_theme(&mut self, theme: usize) {
        if let Some(position) = self.order.iter().position(|&i| i == theme) {
            self.list.select_item(position);
        }
    }

    fn move_by(&mut self, delta: isize) {
        self.list.move_by(delta);
        self.preview();
    }

    fn filter(&mut self, query: &str) {
        self.list.set_query(query);
        self.preview();
    }

    /// Prueba el tema elegido en la lista.
    fn preview(&mut self) {
        if let Some(position) = self.list.selected() {
            self.apply(self.order[position]);
        }
    }

    fn cancel(&mut self) {
        self.apply(self.saved);
        close_self();
    }

    fn apply(&mut self, theme: usize) {
        if theme == self.applied {
            return;
        }
        self.applied = theme;
        if let Some(controller) = self.controller {
            pipe_message_to_plugin(
                MessageToPlugin::new("set_theme").with_destination_plugin_id(controller).with_payload(THEMES[theme].id),
            );
        }
    }
}

fn context(command: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("vkterm".to_string(), command.to_string())])
}
