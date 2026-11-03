//! «Git: Comparar con otra rama…»: la rama con la que la vista Control de
//! código fuente comprueba si habría conflictos al fusionar. Lista las ramas
//! locales y remotas (`vkterm-git branches`); lo escrito filtra. La primera
//! opción vuelve a la rama principal (main, master…), la que se usa si no se
//! elige otra.
//!
//! Es otra instancia del plugin (configuración `mode "branches"`). Solo
//! dibuja: lo elegido vuelve al explorador (controlador) en un mensaje
//! `palette` (`compare_branch`) cuando esta ventana ya se ha cerrado.
use std::collections::BTreeMap;

use crate::floating::{self, Floating, FRAME};
use vkterm::quickpick::{QuickPick, Row};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Comparar con otra rama";
const HINT: &str = "Enter comparar · Esc cancelar";
const PLACEHOLDER: &str = "Rama con la que comprobar los conflictos al fusionar";
const DEFAULT_LABEL: &str = "Rama principal (main, master…)";
const PAD_X: usize = 2;
const LIST_ROWS: usize = 12;
const WIDTH: usize = 70;

#[derive(Default)]
pub struct Branches {
    floating: Floating,
    controller: Option<u32>,
    /// El nombre de cada elemento ("" es la rama principal).
    names: Vec<String>,
    list: QuickPick,
    loaded: bool,
    current: String,
}

impl Branches {
    pub fn load(&mut self) {
        self.floating.load(&[EventType::RunCommandResult]);
    }

    /// payload: la rama con la que se compara ahora ("" si es la principal).
    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "toggle_branches" {
            return false;
        }
        if self.floating.open(TITLE) {
            self.current = message.payload.unwrap_or_default().trim().to_string();
            let cwd = get_plugin_ids().initial_cwd;
            let context = BTreeMap::from([("vkterm".to_string(), "branches".to_string())]);
            run_command_with_env_variables_and_cwd(&["vkterm-git", "branches"], BTreeMap::new(), cwd, context);
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
            Event::RunCommandResult(_, stdout, _, ctx) if ctx.get("vkterm").map(String::as_str) == Some("branches") => {
                let stdout = String::from_utf8_lossy(&stdout);
                let mut items = vec![(DEFAULT_LABEL.to_string(), None)];
                self.names = vec![String::new()];
                for line in stdout.lines() {
                    let Some((kind, name)) = line.split_once('\t') else { continue };
                    let group = if kind == "local" { "ramas locales" } else { "ramas remotas" };
                    items.push((name.to_string(), Some(group)));
                    self.names.push(name.to_string());
                }
                self.list = QuickPick::new(items);
                match self.names.iter().position(|n| *n == self.current) {
                    Some(i) => self.list.select_item(i),
                    None => self.list.first(),
                }
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
                    self.choose();
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
            BareKey::Down if plain => self.list.move_by(1),
            BareKey::Up if plain => self.list.move_by(-1),
            BareKey::PageDown if plain => self.list.move_by(LIST_ROWS as isize),
            BareKey::PageUp if plain => self.list.move_by(-(LIST_ROWS as isize)),
            BareKey::Enter if plain => self.choose(),
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

    fn choose(&mut self) {
        let Some(name) = self.list.selected().map(|i| self.names[i].clone()) else { return };
        if let Some(controller) = self.controller {
            pipe_message_to_plugin(
                MessageToPlugin::new("palette")
                    .with_destination_plugin_id(controller)
                    .with_payload(format!("compare_branch\t{name}")),
            );
        }
        close_self();
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
                    let name = &self.names[*index];
                    let mark = if *name == self.current { "  actual" } else { "" };
                    let label = if index == &0 { DEFAULT_LABEL } else { name.as_str() };
                    let mut line = floating::row(&format!("{label}{mark}"), width, &palette);
                    let current = selected == Some(scroll + y);
                    if current {
                        line.paint(0..width, |_| palette.quick_selection);
                    }
                    let len = label.chars().count();
                    let description = if current { palette.quick_selection.fg } else { palette.quick_description };
                    line.fg(len..width, description);
                    let highlight = if current { palette.quick_selection_highlight } else { palette.quick_highlight };
                    line.highlight(&matched.iter().copied().filter(|&i| i < len).collect::<Vec<_>>(), highlight);
                    line
                }
            };
            print!("{}", line.render(x, FRAME + 1 + y));
        }
        if self.list.rows().is_empty() {
            let message = if self.loaded { "Ninguna rama coincide" } else { "Buscando ramas…" };
            let mut line = floating::row(message, width, &palette);
            line.fg(0..width, palette.quick_description);
            print!("{}", line.render(x, FRAME + 1));
        }
    }
}
