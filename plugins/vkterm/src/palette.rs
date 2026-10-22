//! Paleta (⌘P y ⌘⇧P), como "Quick Open" y "Command Palette" de VS Code en una
//! sola ventana: lo escrito busca archivos del proyecto por nombre y, si
//! empieza por `>`, comandos de vkTERM (src/commands.rs). ⌘P la abre vacía y
//! ⌘⇧P con `>`; repetir el mismo atajo la cierra y el otro cambia de modo.
//!
//! Es otra instancia del plugin (configuración `mode "palette"`), como la
//! ventana de atajos y el selector de tema. Lo elegido se manda al explorador
//! (controlador) en un mensaje `palette`, que lo ejecuta cuando la ventana
//! ya se ha cerrado y el foco ha vuelto a los paneles fijos.
//!
//! Los archivos salen de `git ls-files` (respeta .gitignore); sin git, se
//! recorre el proyecto saltando las carpetas pesadas habituales.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::floating::{self, Floating, FRAME};
use vkterm::commands::COMMANDS;
use vkterm::fuzzy::{self, Match};
use zellij_tile::prelude::*;

pub const FILES_TITLE: &str = "Abrir archivo";
pub const COMMANDS_TITLE: &str = "Paleta de comandos";
const HINT: &str = "↑ ↓ elegir · Enter abrir · Esc cerrar";
const PAD_X: usize = 2;
const LIST_ROWS: usize = 12;
const MAX_FILES: usize = 50_000;
/// Carpetas que no se recorren cuando no hay git (como search.exclude de VS Code).
const SKIP_DIRS: [&str; 8] = [".git", "node_modules", "target", "dist", "build", ".venv", "__pycache__", ".next"];

#[derive(Default)]
pub struct Palette {
    floating: Floating,
    cwd: PathBuf,
    /// Explorador (controlador), que ejecuta lo elegido.
    controller: Option<u32>,
    /// Lo escrito; con `>` delante, modo comandos.
    input: String,
    /// Archivos del proyecto, relativos a su raíz; None mientras se buscan.
    files: Option<Vec<String>>,
    results: Vec<(usize, Match)>,
    selected: usize,
    scroll: usize,
}

impl Palette {
    pub fn load(&mut self) {
        self.floating.load(&[EventType::RunCommandResult]);
        self.cwd = get_plugin_ids().initial_cwd;
    }

    fn commands_mode(&self) -> bool {
        self.input.starts_with('>')
    }

    fn query(&self) -> &str {
        self.input.strip_prefix('>').unwrap_or(&self.input)
    }

    fn title(&self) -> &'static str {
        if self.commands_mode() {
            COMMANDS_TITLE
        } else {
            FILES_TITLE
        }
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        let want = match message.name.as_str() {
            "palette_files" => "",
            "palette_commands" => ">",
            _ => return false,
        };
        if self.floating.open(self.title()) {
            self.input = want.to_string();
            self.load_files();
            self.refresh();
            rename_plugin_pane(self.floating.me, self.title());
        } else if self.commands_mode() == (want == ">") {
            close_self();
        } else {
            self.switch(want);
        }
        true
    }

    /// Cambia de modo sin cerrar la ventana (⌘P sobre la paleta de comandos y
    /// al revés, o el comando "Archivo: Abrir archivo…").
    fn switch(&mut self, input: &str) {
        self.input = input.to_string();
        self.refresh();
        rename_plugin_pane(self.floating.me, self.title());
    }

    fn load_files(&mut self) {
        self.files = None;
        run_command_with_env_variables_and_cwd(
            &["git", "ls-files", "--cached", "--others", "--exclude-standard"],
            BTreeMap::new(),
            self.cwd.clone(),
            BTreeMap::from([("vkterm".to_string(), "files".to_string())]),
        );
    }

    pub fn update(&mut self, event: Event) -> bool {
        if self.floating.lost_focus(&event) {
            close_self();
            return false;
        }
        match event {
            Event::RunCommandResult(code, stdout, _, context)
                if context.get("vkterm").map(String::as_str) == Some("files") =>
            {
                let listed: Vec<String> = String::from_utf8_lossy(&stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .take(MAX_FILES)
                    .map(str::to_string)
                    .collect();
                self.files = Some(if code == Some(0) {
                    listed
                } else {
                    let mut files = Vec::new();
                    walk(Path::new(crate::HOST), "", &mut files);
                    files
                });
                self.refresh();
                true
            }
            Event::PaneUpdate(manifest) => {
                let panes = manifest.panes.values().flatten();
                let url = panes.clone().find(|p| p.is_plugin && p.id == self.floating.me).and_then(|p| p.plugin_url.clone());
                self.controller = panes
                    .filter(|p| p.is_plugin && !p.is_floating && p.id != self.floating.me)
                    .find(|p| url.is_some() && p.plugin_url == url)
                    .map(|p| p.id);
                let (columns, _) = self.floating.area();
                let width = (columns * 3 / 5).clamp(44, 100);
                self.floating.place(width, LIST_ROWS + 1 + 2 * FRAME);
                false
            }
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(Mouse::ScrollDown(_)) => {
                self.select(self.selected + 1);
                true
            }
            Event::Mouse(Mouse::ScrollUp(_)) => {
                self.select(self.selected.saturating_sub(1));
                true
            }
            Event::Mouse(Mouse::LeftClick(line, _)) => {
                let row = line - (FRAME + 1) as isize;
                if row >= 0 && ((row as usize) + self.scroll) < self.results.len() {
                    self.select(row as usize + self.scroll);
                    self.run();
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
            BareKey::Esc => close_self(),
            BareKey::Enter if plain => self.run(),
            BareKey::Down if plain => self.select(self.selected + 1),
            BareKey::Up if plain => self.select(self.selected.saturating_sub(1)),
            BareKey::PageDown if plain => self.select(self.selected + LIST_ROWS),
            BareKey::PageUp if plain => self.select(self.selected.saturating_sub(LIST_ROWS)),
            BareKey::Home if plain => self.select(0),
            BareKey::End if plain => self.select(usize::MAX),
            BareKey::Backspace => {
                self.input.pop();
                self.refresh();
            }
            BareKey::Char('u') if key.has_only_modifiers(&[KeyModifier::Ctrl]) => {
                let keep = if self.commands_mode() { ">" } else { "" };
                self.switch(keep);
            }
            BareKey::Char(c) if typed && !c.is_control() => {
                let was_commands = self.commands_mode();
                self.input.push(c);
                self.refresh();
                if was_commands != self.commands_mode() {
                    rename_plugin_pane(self.floating.me, self.title());
                }
            }
            _ => return false,
        }
        true
    }

    fn refresh(&mut self) {
        self.results = if self.commands_mode() {
            let labels: Vec<&str> = COMMANDS.iter().map(|c| c.label).collect();
            fuzzy::rank(self.query(), &labels)
        } else {
            match &self.files {
                Some(files) => fuzzy::rank(self.query(), files),
                None => Vec::new(),
            }
        };
        self.selected = 0;
        self.scroll = 0;
    }

    fn select(&mut self, i: usize) {
        if self.results.is_empty() {
            return;
        }
        self.selected = i.min(self.results.len() - 1);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + LIST_ROWS {
            self.scroll = self.selected + 1 - LIST_ROWS;
        }
    }

    /// Enter o clic: manda lo elegido al controlador y cierra la ventana.
    fn run(&mut self) {
        let Some((index, _)) = self.results.get(self.selected).cloned() else {
            return;
        };
        if self.commands_mode() {
            let command = COMMANDS[index];
            match command.message {
                "palette_files" => self.switch(""),
                "palette_commands" => self.switch(">"),
                _ => {
                    self.send(command.message, command.payload);
                    close_self();
                }
            }
        } else if let Some(file) = self.files.as_ref().and_then(|f| f.get(index)) {
            let file = file.clone();
            self.send("open", &file);
            close_self();
        }
    }

    fn send(&self, name: &str, payload: &str) {
        if let Some(controller) = self.controller {
            pipe_message_to_plugin(
                MessageToPlugin::new("palette")
                    .with_destination_plugin_id(controller)
                    .with_payload(format!("{name}\t{payload}")),
            );
        }
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let x = FRAME + PAD_X;
        let width = cols.saturating_sub(2 * x);
        if width < 8 || rows < 2 * FRAME + 2 {
            return;
        }
        // Línea de entrada: "› " y lo escrito, con el cursor; sin texto, la ayuda.
        let (prompt, placeholder) = if self.commands_mode() {
            (">", "Escribe un comando")
        } else {
            ("›", "Buscar archivos por nombre")
        };
        let query = self.query().to_string();
        let line = if query.is_empty() {
            format!("{prompt} ▏{placeholder}")
        } else {
            format!("{prompt} {query}▏")
        };
        let line = crate::fit(&line, width);
        let mut text = Text::new(&line).color_range(2, ..1);
        if query.is_empty() {
            text = text.dim_range(3..);
        }
        print_text_with_coordinates(text, x, FRAME, Some(width), Some(1));

        let list_rows = rows.saturating_sub(2 * FRAME + 1);
        let keys_width = COMMANDS.iter().map(|c| c.keys.chars().count()).max().unwrap_or(0);
        for y in 0..list_rows {
            let i = self.scroll + y;
            let Some((index, m)) = self.results.get(i) else {
                let message = match (&self.files, y, self.results.is_empty()) {
                    (None, 0, _) if !self.commands_mode() => "Buscando archivos…",
                    (_, 0, true) => "Sin resultados",
                    _ => "",
                };
                let blank = crate::fit(message, width);
                print_text_with_coordinates(Text::new(&blank).dim_all(), x, FRAME + 1 + y, Some(width), Some(1));
                continue;
            };
            let (label, keys) = if self.commands_mode() {
                (COMMANDS[*index].label, COMMANDS[*index].keys)
            } else {
                (self.files.as_ref().map_or("", |f| f[*index].as_str()), "")
            };
            let label_width = if self.commands_mode() { width.saturating_sub(keys_width + 2) } else { width };
            let shown = crate::fit(label, label_width);
            let shown_chars = shown.chars().count();
            let line = if self.commands_mode() {
                format!("{shown}  {keys:>keys_width$}")
            } else {
                shown
            };
            let indices: Vec<usize> = m.indices.iter().copied().filter(|&i| i < label_width.min(shown_chars)).collect();
            let mut text = Text::new(&line).color_indices(0, indices);
            if self.commands_mode() {
                text = text.dim_range(label_width + 2..);
            }
            if i == self.selected {
                text = text.selected();
            }
            print_text_with_coordinates(text, x, FRAME + 1 + y, Some(width), Some(1));
        }
        floating::frame(rows, cols, self.title(), HINT);
    }
}

/// Recorre el proyecto (sin git): archivos relativos a la raíz, en orden.
fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    if out.len() >= MAX_FILES {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        if entry.path().is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) {
                walk(&entry.path(), &relative, out);
            }
        } else {
            out.push(relative);
        }
    }
}
