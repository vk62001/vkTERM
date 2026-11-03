//! «Abrir reciente» (⌘O), como «File: Open Recent» de VS Code: las carpetas
//! ya abiertas en vkTERM, la última usada primero (bin/vkterm-state recent),
//! sin la de este IDE; lo escrito filtra. Enter abre la elegida en esta
//! pestaña (si ya está abierta en otra, bin/vkterm trae aquí su IDE) y ⇧Enter
//! en una ventana nueva (si ya está abierta, se va a ella).
//!
//! Es otra instancia del plugin (configuración `mode "recent"`). Solo dibuja:
//! lo elegido lo hace el explorador (controlador), que recibe un mensaje
//! `open_recent` cuando esta ventana ya se ha cerrado. Para abrir en esta
//! pestaña, cierra antes la carpeta actual, como ⌘K F (pregunta si se
//! perdería algo).
use std::collections::BTreeMap;

use crate::floating::{self, Floating, FRAME};
use vkterm::folders::{self, Folder, Mark};
use vkterm::quickpick::{QuickPick, Row};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Abrir reciente";
const HINT: &str = "Enter abrir aquí · ⇧Enter ventana nueva · Esc cancelar";
const PLACEHOLDER: &str = "Escribe para filtrar las carpetas";
const MISSING: &str = "Esa carpeta ya no existe (¿está conectado su disco?)";
const PAD_X: usize = 2;
const LIST_ROWS: usize = 12;
const WIDTH: usize = 80;
/// La carpeta personal (para mostrar ~) y la lista, en una sola orden.
const LIST: &str = "printf '%s\\n' \"$HOME\"; exec vkterm-state recent";

#[derive(Default)]
pub struct Recent {
    floating: Floating,
    controller: Option<u32>,
    folders: Vec<Folder>,
    list: QuickPick,
    loaded: bool,
    message: Option<&'static str>,
}

impl Recent {
    pub fn load(&mut self) {
        self.floating.load(&[EventType::RunCommandResult]);
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "toggle_recent" {
            return false;
        }
        if self.floating.open(TITLE) {
            run_command(&["sh", "-c", LIST], context("list"));
        } else {
            close_self();
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
            Event::RunCommandResult(_, stdout, _, ctx) if ctx.get("vkterm").map(String::as_str) == Some("list") => {
                let stdout = String::from_utf8_lossy(&stdout);
                let (home, list) = stdout.split_once('\n').unwrap_or((&stdout, ""));
                self.folders = folders::parse(list, home.trim());
                self.list = QuickPick::new(self.folders.iter().map(|f| (f.label(), None)).collect());
                self.list.first();
                self.loaded = true;
                true
            }
            Event::PaneUpdate(manifest) => {
                self.controller = floating::controller(&manifest, self.floating.me);
                self.floating.place(WIDTH, LIST_ROWS + 1 + 2 * FRAME);
                false
            }
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(Mouse::ScrollDown(_)) => {
                self.list.move_by(1);
                true
            }
            Event::Mouse(Mouse::ScrollUp(_)) => {
                self.list.move_by(-1);
                true
            }
            Event::Mouse(Mouse::LeftClick(line, _)) => {
                let row = line - (FRAME + 1) as isize;
                if row >= 0 && self.list.select_row(self.list.scroll() + row as usize) {
                    self.choose("here");
                }
                true
            }
            _ => false,
        }
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        let plain = key.has_no_modifiers();
        let typed = plain || key.has_only_modifiers(&[KeyModifier::Shift]);
        self.message = None;
        match key.bare_key {
            BareKey::Down if plain => self.list.move_by(1),
            BareKey::Up if plain => self.list.move_by(-1),
            BareKey::PageDown if plain => self.list.move_by(LIST_ROWS as isize),
            BareKey::PageUp if plain => self.list.move_by(-(LIST_ROWS as isize)),
            BareKey::Enter if plain => self.choose("here"),
            // ⇧Enter (y ⌥Enter, si llega como Alt): en una ventana nueva.
            BareKey::Enter
                if key.has_only_modifiers(&[KeyModifier::Shift]) || key.has_only_modifiers(&[KeyModifier::Alt]) =>
            {
                self.choose("window")
            }
            BareKey::Esc => close_self(),
            BareKey::Backspace => {
                let mut query = self.list.query().to_string();
                query.pop();
                self.list.set_query(&query);
            }
            BareKey::Char('u') if key.has_only_modifiers(&[KeyModifier::Ctrl]) => self.list.set_query(""),
            BareKey::Char(c) if typed && !c.is_control() => {
                let query = format!("{}{c}", self.list.query());
                self.list.set_query(&query);
            }
            _ => return false,
        }
        true
    }

    /// "here": en esta pestaña; "window": en una ventana nueva.
    fn choose(&mut self, place: &str) {
        let Some(folder) = self.list.selected().map(|i| &self.folders[i]) else { return };
        if folder.mark == Mark::Missing {
            self.message = Some(MISSING);
            return;
        }
        if let Some(controller) = self.controller {
            pipe_message_to_plugin(
                MessageToPlugin::new("palette")
                    .with_destination_plugin_id(controller)
                    .with_payload(format!("open_recent\t{place}\t{}", folder.path)),
            );
        }
        close_self();
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let palette = self.floating.palette;
        floating::frame(rows, cols, TITLE, self.message.unwrap_or(HINT), &palette);
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
            let Row::Item { index, matched } = row else { continue };
            let folder = &self.folders[*index];
            let mark = folder.mark.text();
            let label_width = if mark.is_empty() { width } else { width.saturating_sub(mark.chars().count() + 2) };
            let shown = crate::fit(&folder.label(), label_width);
            let mut line = floating::row(&format!("{shown}  {mark}"), width, &palette);
            let current = selected == Some(scroll + y);
            if current {
                line.paint(0..width, |_| palette.quick_selection);
            }
            let name = folder.name.chars().count();
            let description = if current { palette.quick_selection.fg } else { palette.quick_description };
            let dim_from = if folder.mark == Mark::Missing { 0 } else { name };
            line.fg(dim_from..width, description);
            let shown_chars = shown.trim_end().chars().count();
            let indices: Vec<usize> = matched.iter().copied().filter(|&i| i < shown_chars).collect();
            line.highlight(&indices, if current { palette.quick_selection_highlight } else { palette.quick_highlight });
            print!("{}", line.render(x, FRAME + 1 + y));
        }
        if self.list.rows().is_empty() {
            let message = match (self.loaded, self.folders.is_empty()) {
                (false, _) => "Buscando carpetas…",
                (true, true) => "Aún no hay otras carpetas abiertas en vkTERM",
                (true, false) => "Ninguna carpeta coincide",
            };
            let mut line = floating::row(message, width, &palette);
            line.fg(0..width, palette.quick_description);
            print!("{}", line.render(x, FRAME + 1));
        }
    }
}

fn context(command: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("vkterm".to_string(), command.to_string())])
}
