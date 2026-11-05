//! Buscar en el proyecto (⌘⇧F), como la vista «Search» de VS Code en una
//! ventana flotante: lo escrito se busca (ripgrep; `git grep` si no está) en
//! todos los archivos del proyecto que respeta .gitignore; ↑ ↓ eligen un
//! resultado y Enter lo abre en el editor, en esa línea.
//!
//! Tab añade el campo «Reemplazar»: Enter en él pide confirmación (con el
//! número de archivos) y la segunda vez reemplaza el texto, literal, en todos.
//! Los archivos abiertos en el editor con cambios sin guardar no se avisan:
//! Micro ofrece recargarlos.
//!
//! Es otra instancia del plugin (`mode "search"`), como la paleta; lo elegido
//! se manda al explorador en un mensaje `palette` (`open`).
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::floating::{self, Floating, FRAME};
use vkterm::grep::{self, Hit};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Buscar en el proyecto";
const HINT: &str = "↑ ↓ elegir · Enter abrir · Tab reemplazar · Esc cerrar";
const REPLACE_HINT: &str = "Enter reemplazar · Tab volver a buscar · Esc cerrar";
const PAD_X: usize = 2;
const LIST_ROWS: usize = 14;
const MAX_HITS: usize = 500;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Search,
    Replace,
}

pub struct Search {
    floating: Floating,
    cwd: PathBuf,
    controller: Option<u32>,
    query: String,
    replacement: String,
    field: Field,
    hits: Vec<Hit>,
    /// La búsqueda en curso o lo que falló.
    note: String,
    selected: usize,
    scroll: usize,
    /// Archivos que se reemplazarían, tras el primer Enter.
    pending: Option<usize>,
    replace_mode: bool,
}

impl Default for Search {
    fn default() -> Self {
        Search {
            floating: Floating::default(),
            cwd: PathBuf::new(),
            controller: None,
            query: String::new(),
            replacement: String::new(),
            field: Field::Search,
            hits: Vec::new(),
            note: String::new(),
            selected: 0,
            scroll: 0,
            pending: None,
            replace_mode: false,
        }
    }
}

impl Search {
    pub fn load(&mut self) {
        self.floating.load(&[EventType::RunCommandResult]);
        self.cwd = get_plugin_ids().initial_cwd;
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "toggle_search" {
            return false;
        }
        if self.floating.open(TITLE) {
            rename_plugin_pane(self.floating.me, TITLE);
        } else {
            close_self();
        }
        true
    }

    fn search(&mut self) {
        self.pending = None;
        self.hits.clear();
        self.selected = 0;
        self.scroll = 0;
        if self.query.chars().count() < 2 {
            self.note.clear();
            return;
        }
        self.note = "Buscando…".to_string();
        // ripgrep respeta .gitignore y salta los binarios; `git grep` es la
        // alternativa si no está instalado.
        let context = BTreeMap::from([("vkterm".to_string(), "grep".to_string()), ("q".to_string(), self.query.clone())]);
        let script = "if command -v rg >/dev/null 2>&1; then \
            rg --line-number --column --no-heading --color=never --smart-case --fixed-strings \
               --max-count 50 --max-columns 300 --max-filesize 2M -e \"$Q\"; \
            else git grep -n -I -F --column ${SMART:+-i} -e \"$Q\" 2>/dev/null; fi | head -n 500";
        let mut env = BTreeMap::from([("Q".to_string(), self.query.clone())]);
        if grep::ignore_case(&self.query) {
            env.insert("SMART".to_string(), "1".to_string());
        }
        run_command_with_env_variables_and_cwd(&["sh", "-c", script], env, self.cwd.clone(), context);
    }

    fn count_files(&mut self) {
        let context = BTreeMap::from([("vkterm".to_string(), "count".to_string()), ("q".to_string(), self.query.clone())]);
        let script = "if command -v rg >/dev/null 2>&1; then rg -l --smart-case --fixed-strings -e \"$Q\"; \
            else git grep -l -I -F ${SMART:+-i} -e \"$Q\"; fi 2>/dev/null | wc -l";
        let mut env = BTreeMap::from([("Q".to_string(), self.query.clone())]);
        if grep::ignore_case(&self.query) {
            env.insert("SMART".to_string(), "1".to_string());
        }
        run_command_with_env_variables_and_cwd(&["sh", "-c", script], env, self.cwd.clone(), context);
    }

    fn replace_all(&mut self) {
        let script = format!(
            "if command -v rg >/dev/null 2>&1; then rg -l0 --smart-case --fixed-strings -e \"$Q\"; \
             else git grep -l -z -I -F ${{SMART:+-i}} -e \"$Q\"; fi 2>/dev/null \
             | xargs -0 perl -pi -e '{}'",
            grep::REPLACE_PERL
        );
        let mut env = BTreeMap::from([("Q".to_string(), self.query.clone()), ("R".to_string(), self.replacement.clone())]);
        if grep::ignore_case(&self.query) {
            env.insert("SMART".to_string(), "1".to_string());
        }
        let context = BTreeMap::from([("vkterm".to_string(), "replace".to_string())]);
        run_command_with_env_variables_and_cwd(&["sh", "-c", &script], env, self.cwd.clone(), context);
        self.note = "Reemplazando…".to_string();
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
            Event::RunCommandResult(_, stdout, _, context) => {
                let kind = context.get("vkterm").map(String::as_str);
                let same = context.get("q").map_or(true, |q| *q == self.query);
                match kind {
                    Some("grep") if same => {
                        self.hits = String::from_utf8_lossy(&stdout)
                            .lines()
                            .filter_map(grep::parse_line)
                            .take(MAX_HITS)
                            .collect();
                        self.note = if self.hits.is_empty() { "Sin resultados".to_string() } else { String::new() };
                    }
                    Some("count") if same => {
                        let files: usize = String::from_utf8_lossy(&stdout).trim().parse().unwrap_or(0);
                        if files == 0 {
                            self.note = "Nada que reemplazar".to_string();
                        } else {
                            self.pending = Some(files);
                        }
                    }
                    Some("replace") => {
                        self.pending = None;
                        close_self();
                    }
                    _ => return false,
                }
                true
            }
            Event::PaneUpdate(manifest) => {
                self.controller = floating::controller(&manifest, self.floating.me);
                let (columns, _) = self.floating.area();
                let width = (columns * 4 / 5).clamp(50, 130);
                self.floating.place(width, LIST_ROWS + 3 + 2 * FRAME);
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
                let row = line - (FRAME + 2 + usize::from(self.replace_mode) ) as isize;
                if row >= 0 && ((row as usize) + self.scroll) < self.hits.len() {
                    self.select(row as usize + self.scroll);
                    self.open();
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
            BareKey::Tab if plain => {
                self.replace_mode = true;
                self.field = if self.field == Field::Search { Field::Replace } else { Field::Search };
                self.pending = None;
            }
            BareKey::Enter if plain => match self.field {
                Field::Search => self.open(),
                Field::Replace => {
                    if self.query.chars().count() < 2 {
                        return true;
                    }
                    if self.pending.is_some() {
                        self.replace_all();
                    } else {
                        self.count_files();
                    }
                }
            },
            BareKey::Down if plain => self.select(self.selected + 1),
            BareKey::Up if plain => self.select(self.selected.saturating_sub(1)),
            BareKey::PageDown if plain => self.select(self.selected + LIST_ROWS),
            BareKey::PageUp if plain => self.select(self.selected.saturating_sub(LIST_ROWS)),
            BareKey::Backspace => {
                self.edit(|s| {
                    s.pop();
                });
            }
            BareKey::Char('u') if key.has_only_modifiers(&[KeyModifier::Ctrl]) => self.edit(String::clear),
            BareKey::Char(c) if typed && !c.is_control() => self.edit(|s| s.push(c)),
            _ => return false,
        }
        true
    }

    fn edit(&mut self, change: impl FnOnce(&mut String)) {
        match self.field {
            Field::Search => {
                change(&mut self.query);
                self.search();
            }
            Field::Replace => {
                change(&mut self.replacement);
                self.pending = None;
            }
        }
    }

    fn select(&mut self, i: usize) {
        if self.hits.is_empty() {
            return;
        }
        self.selected = i.min(self.hits.len() - 1);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + LIST_ROWS {
            self.scroll = self.selected + 1 - LIST_ROWS;
        }
    }

    fn open(&mut self) {
        let Some(hit) = self.hits.get(self.selected) else { return };
        if let Some(controller) = self.controller {
            pipe_message_to_plugin(
                MessageToPlugin::new("palette")
                    .with_destination_plugin_id(controller)
                    .with_payload(format!("open\t{}\t{}", hit.path, hit.line)),
            );
        }
        close_self();
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let palette = self.floating.palette;
        let hint = if self.field == Field::Replace { REPLACE_HINT } else { HINT };
        floating::frame(rows, cols, TITLE, hint, &palette);
        let x = FRAME + PAD_X;
        let width = cols.saturating_sub(2 * x);
        if width < 8 || rows < 2 * FRAME + 3 {
            return;
        }
        let shown = |field: Field, prompt: &str, text: &str, placeholder: &str| {
            let mut line = if self.field == field {
                floating::input_line(prompt, text, placeholder, width, &palette)
            } else {
                let mut l = floating::row(&format!("{prompt} {}", if text.is_empty() { placeholder } else { text }), width, &palette);
                l.fg(0..width, palette.quick_description);
                l
            };
            line.fg(0..1, palette.accent);
            line
        };
        print!("{}", shown(Field::Search, "›", &self.query, "Buscar en todos los archivos").render(x, FRAME));
        let mut top = 1;
        if self.replace_mode {
            print!("{}", shown(Field::Replace, "↳", &self.replacement, "Reemplazar por").render(x, FRAME + 1));
            top = 2;
        }
        let status = match self.pending {
            Some(n) => format!("Enter otra vez: reemplazar en {n} archivo{}", if n == 1 { "" } else { "s" }),
            None if !self.note.is_empty() => self.note.clone(),
            None if !self.hits.is_empty() => {
                let n = grep::files(&self.hits).len();
                format!("{} resultado{} en {n} archivo{}", self.hits.len(), if self.hits.len() == 1 { "" } else { "s" }, if n == 1 { "" } else { "s" })
            }
            None => String::new(),
        };
        let mut line = floating::row(&crate::fit(&status, width), width, &palette);
        line.fg(0..width, palette.quick_description);
        print!("{}", line.render(x, FRAME + top));

        let list_rows = rows.saturating_sub(2 * FRAME + top + 1);
        for y in 0..list_rows {
            let i = self.scroll + y;
            let Some(hit) = self.hits.get(i) else {
                print!("{}", floating::row("", width, &palette).render(x, FRAME + top + 1 + y));
                continue;
            };
            let mut line = floating::row(&crate::fit(&grep::row_text(hit, usize::MAX), width), width, &palette);
            let head = format!("{}:{}", hit.path, hit.line).chars().count().min(width);
            if i == self.selected {
                line.paint(0..width, |_| palette.quick_selection);
            } else {
                line.fg(0..head, palette.quick_description);
            }
            print!("{}", line.render(x, FRAME + top + 1 + y));
        }
    }
}
