//! Selector de fuente (⌘K ⌘L), como el de tema: las fuentes monoespaciadas
//! instaladas (`ghostty +list-fonts`), lo escrito filtra y al moverse por la
//! lista se prueba cada una en toda la ventana; ← y → cambian el tamaño.
//! Enter la deja y Esc vuelve a la fuente y al tamaño que había (también al
//! cerrarlo con el atajo o al perder el foco).
//!
//! Es otra instancia del plugin (configuración `mode "fonts"`). Solo dibuja:
//! la fuente la aplica el explorador (controlador) con bin/vkterm-font, que
//! recibe cada cambio en un mensaje `set_font` o `font_size`.
use std::collections::BTreeMap;

use crate::floating::{self, Floating, FRAME};
use vkterm::quickpick::{QuickPick, Row};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Fuente";
const HINT: &str = "↑ ↓ probar · ← → tamaño · Enter aplicar · Esc cancelar";
const PLACEHOLDER: &str = "Escribe para filtrar las fuentes";
const PAD_X: usize = 2;
const LIST_ROWS: usize = 14;
const MIN_SIZE: i32 = 6;
const MAX_SIZE: i32 = 48;

#[derive(Default)]
pub struct Fonts {
    floating: Floating,
    controller: Option<u32>,
    families: Vec<String>,
    list: QuickPick,
    /// Fuente y tamaño guardados al abrir.
    saved: String,
    saved_size: i32,
    /// Fuente aplicada ahora mismo y tamaño que se ve.
    applied: String,
    size: i32,
    loaded: bool,
}

impl Fonts {
    pub fn load(&mut self) {
        self.floating.load(&[EventType::RunCommandResult]);
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "toggle_fonts" {
            return false;
        }
        if self.floating.open(TITLE) {
            run_command(&["vkterm-font", "current"], context("current"));
            run_command(&["vkterm-font", "list"], context("list"));
        } else {
            self.cancel();
        }
        true
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
            Event::RunCommandResult(_, stdout, _, ctx) => {
                let stdout = String::from_utf8_lossy(&stdout);
                match ctx.get("vkterm").map(String::as_str) {
                    // "familia\ttamaño" (familia vacía: la de Ghostty por defecto).
                    Some("current") => {
                        let line = stdout.lines().next().unwrap_or_default();
                        let (family, size) = line.split_once('\t').unwrap_or((line, ""));
                        self.saved = family.to_string();
                        self.applied = self.saved.clone();
                        self.saved_size = size.trim().parse().unwrap_or(13);
                        self.size = self.saved_size;
                    }
                    Some("list") => {
                        self.families = stdout.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
                        self.list = QuickPick::new(self.families.iter().map(|f| (f.clone(), None)).collect());
                        self.loaded = true;
                    }
                    _ => return false,
                }
                if let Some(i) = self.families.iter().position(|f| *f == self.saved) {
                    self.list.select_item(i);
                }
                true
            }
            Event::PaneUpdate(manifest) => {
                self.controller = floating::controller(&manifest, self.floating.me);
                let names = self.families.iter().map(|f| f.chars().count()).max().unwrap_or(24);
                let width = (names + 12 + 2 * (FRAME + PAD_X)).max(HINT.chars().count() + 4);
                self.floating.place(width, LIST_ROWS + 2 + 2 * FRAME);
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
                let row = line - (FRAME + 2) as isize;
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
            BareKey::Right if plain => self.resize(1),
            BareKey::Left if plain => self.resize(-1),
            BareKey::Enter if plain => close_self(),
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
        if width < 8 || rows < 2 * FRAME + 3 {
            return;
        }
        print!("{}", floating::input_line("›", self.list.query(), PLACEHOLDER, width, &palette).render(x, FRAME));
        let mut size = floating::row(&format!("Tamaño: {} pt   ← más pequeña · → más grande", self.size), width, &palette);
        size.fg(0..7, palette.quick_description);
        size.fg(11 + self.size.to_string().len()..width, palette.quick_description);
        print!("{}", size.render(x, FRAME + 1));
        self.list.set_height(rows.saturating_sub(2 * FRAME + 2));
        let selected = self.list.selected_row();
        let scroll = self.list.scroll();
        for (y, row) in self.list.visible().iter().enumerate() {
            let Row::Item { index, matched } = row else { continue };
            let family = &self.families[*index];
            let mark = if *family == self.saved { "● " } else { "  " };
            let mut line = floating::row(&format!("{mark}{family}"), width, &palette);
            let current = selected == Some(scroll + y);
            if current {
                line.paint(0..width, |_| palette.quick_selection);
            }
            line.fg(0..1, palette.accent);
            let shifted: Vec<usize> = matched.iter().map(|i| i + 2).collect();
            line.highlight(&shifted, if current { palette.quick_selection_highlight } else { palette.quick_highlight });
            print!("{}", line.render(x, FRAME + 2 + y));
        }
        if self.list.rows().is_empty() {
            let message = if self.loaded { "No hay fuentes que coincidan" } else { "Buscando fuentes…" };
            let mut line = floating::row(message, width, &palette);
            line.fg(0..width, palette.quick_description);
            print!("{}", line.render(x, FRAME + 2));
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

    fn preview(&mut self) {
        if let Some(i) = self.list.selected() {
            let family = self.families[i].clone();
            self.apply(&family);
        }
    }

    fn resize(&mut self, delta: i32) {
        let size = (self.size + delta).clamp(MIN_SIZE, MAX_SIZE);
        if size != self.size {
            self.send("font_size", &format!("{:+}", size - self.size));
            self.size = size;
        }
    }

    fn cancel(&mut self) {
        if self.loaded {
            let saved = self.saved.clone();
            self.apply(&saved);
            if self.size != self.saved_size {
                self.send("font_size", &format!("{:+}", self.saved_size - self.size));
                self.size = self.saved_size;
            }
        }
        close_self();
    }

    fn apply(&mut self, family: &str) {
        if family == self.applied {
            return;
        }
        self.applied = family.to_string();
        self.send("set_font", family);
    }

    fn send(&self, name: &str, payload: &str) {
        if let Some(controller) = self.controller {
            pipe_message_to_plugin(MessageToPlugin::new(name).with_destination_plugin_id(controller).with_payload(payload));
        }
    }
}

fn context(command: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("vkterm".to_string(), command.to_string())])
}
