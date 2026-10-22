//! Selector de tema (⌘K ⌘T), como "Preferences: Color Theme" de VS Code: al
//! moverse por la lista se prueba cada tema; Enter lo deja y Esc vuelve al que
//! había (también al cerrarlo con el atajo o un clic fuera, o al perder el foco).
//!
//! Es otra instancia del plugin (configuración `mode "themes"`), como la
//! ventana de atajos. Aplicar un tema:
//! - bin/vkterm-theme lo guarda y cambia Ghostty y Micro;
//! - `reconfigure` cambia el de Zellij en esta sesión;
//! - el explorador (controlador) recibe el id para los colores del atenuado.
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::floating::{self, Floating, FRAME};
use vkterm::themes::{self, THEMES};
use zellij_tile::prelude::*;

const TITLE: &str = "Tema de color";
const HINT: &str = "↑ ↓ probar · Enter aplicar · Esc cancelar";
const PAD_X: usize = 2;
const PAD_Y: usize = 1;

#[derive(Default)]
pub struct Picker {
    floating: Floating,
    cwd: PathBuf,
    session: Option<String>,
    /// Explorador (controlador), que pone los colores del atenuado.
    controller: Option<u32>,
    /// Tema guardado al abrir: al que se vuelve si no se confirma.
    saved: usize,
    /// Tema aplicado ahora mismo (el guardado o el que se está probando).
    applied: usize,
    selected: usize,
}

fn index(id: &str) -> usize {
    let id = themes::find(id).id;
    THEMES.iter().position(|t| t.id == id).unwrap_or(0)
}

impl Picker {
    pub fn load(&mut self) {
        self.floating.load(&[EventType::ModeUpdate, EventType::RunCommandResult]);
        self.cwd = get_plugin_ids().initial_cwd;
        self.saved = index(themes::DEFAULT);
        self.applied = self.saved;
        self.selected = self.saved;
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
        match event {
            Event::ModeUpdate(mode) => {
                self.session = mode.session_name;
                false
            }
            Event::RunCommandResult(_, stdout, _, ctx) if ctx.get("vkterm").map(String::as_str) == Some("current") => {
                self.saved = index(&String::from_utf8_lossy(&stdout));
                self.applied = self.saved;
                self.selected = self.saved;
                true
            }
            Event::PaneUpdate(manifest) => {
                let panes = manifest.panes.values().flatten();
                let url = panes.clone().find(|p| p.is_plugin && p.id == self.floating.me).and_then(|p| p.plugin_url.clone());
                self.controller = panes
                    .filter(|p| p.is_plugin && !p.is_floating && p.id != self.floating.me)
                    .find(|p| url.is_some() && p.plugin_url == url)
                    .map(|p| p.id);
                let names = THEMES.iter().map(|t| t.name.chars().count()).max().unwrap_or(0);
                let width = (names + 12 + 2 * (FRAME + PAD_X)).max(HINT.chars().count() + 4);
                self.floating.place(width, THEMES.len() + 2 * (FRAME + PAD_Y));
                false
            }
            Event::Key(key) if key.has_no_modifiers() => {
                match key.bare_key {
                    BareKey::Down => self.select(self.selected + 1),
                    BareKey::Up => self.select(self.selected.saturating_sub(1)),
                    BareKey::Home => self.select(0),
                    BareKey::End => self.select(THEMES.len() - 1),
                    BareKey::Enter => close_self(),
                    BareKey::Esc => self.cancel(),
                    BareKey::Char(c) => {
                        // Como en el explorador: salta al siguiente que empieza por esa letra.
                        let c = c.to_lowercase().next().unwrap_or(c);
                        let starts = |i: &usize| THEMES[*i].name.to_lowercase().starts_with(c);
                        let next = (self.selected + 1..THEMES.len()).chain(0..=self.selected).find(starts);
                        if let Some(i) = next {
                            self.select(i);
                        }
                    }
                    _ => {}
                }
                true
            }
            Event::Mouse(Mouse::ScrollDown(_)) => {
                self.select(self.selected + 1);
                true
            }
            Event::Mouse(Mouse::ScrollUp(_)) => {
                self.select(self.selected.saturating_sub(1));
                true
            }
            Event::Mouse(Mouse::LeftClick(line, _)) => {
                let row = line - (FRAME + PAD_Y) as isize;
                if (0..THEMES.len() as isize).contains(&row) {
                    self.select(row as usize);
                    close_self();
                } else {
                    self.cancel();
                }
                true
            }
            _ => false,
        }
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let width = cols.saturating_sub(2 * (FRAME + PAD_X));
        let names = THEMES.iter().map(|t| t.name.chars().count()).max().unwrap_or(0);
        for (i, theme) in THEMES.iter().enumerate().take(rows.saturating_sub(2 * (FRAME + PAD_Y))) {
            let mark = if i == self.saved { "●" } else { " " };
            let kind = if theme.dark { "oscuro" } else { "claro" };
            let padding = " ".repeat(names - theme.name.chars().count());
            let line = format!("{mark} {}{padding}   {kind}", theme.name);
            let mut text = Text::new(&line).color_range(2, ..1).dim_range(line.chars().count() - kind.len()..);
            if i == self.selected {
                text = text.selected();
            }
            print_text_with_coordinates(text, FRAME + PAD_X, FRAME + PAD_Y + i, Some(width), Some(1));
        }
        floating::frame(rows, cols, TITLE, HINT);
    }

    fn select(&mut self, i: usize) {
        self.selected = i.min(THEMES.len() - 1);
        self.apply(self.selected);
    }

    fn cancel(&mut self) {
        self.apply(self.saved);
        close_self();
    }

    fn apply(&mut self, i: usize) {
        if i == self.applied {
            return;
        }
        self.applied = i;
        let theme = &THEMES[i];
        let mut env = BTreeMap::new();
        if let Some(session) = &self.session {
            env.insert("ZELLIJ_SESSION_NAME".to_string(), session.clone());
        }
        run_command_with_env_variables_and_cwd(
            &["vkterm-theme", "set", theme.id, theme.ghostty, theme.zellij, theme.micro()],
            env,
            self.cwd.clone(),
            context("set"),
        );
        reconfigure(format!("theme \"{}\"", theme.zellij), false);
        if let Some(controller) = self.controller {
            pipe_message_to_plugin(
                MessageToPlugin::new("set_theme").with_destination_plugin_id(controller).with_payload(theme.id),
            );
        }
    }
}

fn context(command: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("vkterm".to_string(), command.to_string())])
}
