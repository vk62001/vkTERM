//! Vista «Buscar» (⌘⇧F), como "Search" de VS Code, en la misma barra lateral
//! que el explorador y Control de código fuente (src/main.rs).
//!
//! - Campo de búsqueda: al escribir se busca (`bin/vkterm-search`, con
//!   ripgrep) en toda la carpeta, respetando .gitignore, y los resultados se
//!   agrupan por archivo. Ctrl t distingue mayúsculas, Ctrl w busca palabras
//!   completas y Ctrl r usa una expresión regular.
//! - Campo de reemplazo (Tab o ↓ lo muestra): Enter reemplaza en todos los
//!   archivos con coincidencias, preguntando antes.
//! - Enter (o un clic) en una coincidencia abre el archivo en esa línea;
//!   en un archivo, pliega o despliega sus coincidencias.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vkterm::question::Question;
use vkterm::search::{self, Results, Row};
use vkterm::style::{fit, Line, Look, Palette, Rgb};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Buscar";
const HEADER_ROWS: usize = 1;
const FOOTER_ROWS: usize = 1;

pub enum Request {
    Ignored,
    Redraw,
    /// Abrir en el editor (ruta relativa al proyecto, línea y columna).
    Open(PathBuf, usize, usize),
}

#[derive(Default)]
pub struct Find {
    query: String,
    replacement: String,
    show_replace: bool,
    case_sensitive: bool,
    regex: bool,
    word: bool,
    results: Results,
    collapsed: BTreeSet<String>,
    index: usize,
    scroll: usize,
    searched: bool,
    running: bool,
    /// Se escribió algo mientras buscaba: otra búsqueda al terminar.
    dirty: bool,
    /// Reemplazo pendiente de la respuesta de la confirmación.
    asking: bool,
    notice: Option<String>,
    info: Option<String>,
    rows: usize,
}

impl Find {
    pub fn focus_query(&mut self) {
        self.index = 0;
        self.scroll = 0;
    }

    // ------------------------------------------------------------ búsqueda

    fn flags(&self) -> Vec<&'static str> {
        let mut flags = Vec::new();
        if self.case_sensitive {
            flags.push("-c");
        }
        if self.regex {
            flags.push("-r");
        }
        if self.word {
            flags.push("-w");
        }
        flags
    }

    pub fn tick(&mut self, project: &Path) {
        if self.dirty && !self.running {
            self.search(project);
        }
    }

    /// Vuelve a buscar (tras escribir, cambiar una opción o reemplazar).
    pub fn search(&mut self, project: &Path) {
        if self.query.is_empty() {
            self.results = Results::default();
            self.searched = false;
            self.dirty = false;
            self.index = self.index.min(self.rows_now().len().saturating_sub(1));
            return;
        }
        if self.running {
            self.dirty = true;
            return;
        }
        self.running = true;
        self.dirty = false;
        let mut args = vec!["vkterm-search"];
        args.extend(self.flags());
        args.extend(["--", &self.query]);
        run_command_with_env_variables_and_cwd(&args, BTreeMap::new(), project.to_path_buf(), crate::context("search"));
    }

    pub fn result(&mut self, code: Option<i32>, stdout: &str, stderr: &str, project: &Path) {
        self.running = false;
        if self.dirty {
            self.search(project);
            return;
        }
        if code == Some(0) || code == Some(1) {
            self.results = Results::parse(stdout);
            self.searched = true;
            self.notice = None;
        } else {
            self.results = Results::default();
            let detail = stderr.lines().next().unwrap_or("");
            self.notice = Some(format!("Error {}: {detail}", code.map_or("?".to_string(), |c| c.to_string())));
        }
        self.index = self.index.min(self.rows_now().len().saturating_sub(1));
    }

    /// «Reemplazar todo»: pregunta antes de tocar los archivos.
    fn ask_replace(&mut self) {
        if self.query.is_empty() || self.results.total == 0 {
            self.notice = Some("No hay coincidencias que reemplazar".to_string());
            return;
        }
        let (n, f) = (self.results.total, self.results.files.len());
        let lines = [
            format!("¿Reemplazar «{}» por «{}»?", self.query, self.replacement),
            format!("{n} {} en {f} {}.", if n == 1 { "coincidencia" } else { "coincidencias" }, if f == 1 { "archivo" } else { "archivos" }),
            "Los archivos se reescriben en el disco; git permite deshacerlo.".to_string(),
        ];
        self.asking = true;
        crate::launch_window(
            "ask",
            "confirm",
            Some(Question::confirm("Reemplazar en los archivos", &lines, "Reemplazar todo", "search_answer", "yes").to_text()),
        );
    }

    /// La respuesta de la ventana de confirmación.
    pub fn answer(&mut self, project: &Path) {
        if !std::mem::take(&mut self.asking) {
            return;
        }
        let mut args = vec!["vkterm-search"];
        args.extend(self.flags());
        args.extend(["--replace", &self.replacement, "--", &self.query]);
        run_command_with_env_variables_and_cwd(&args, BTreeMap::new(), project.to_path_buf(), crate::context("replace"));
    }

    pub fn replace_result(&mut self, code: Option<i32>, stdout: &str, stderr: &str, project: &Path) {
        if code == Some(0) {
            let n: usize = stdout.trim().parse().unwrap_or(0);
            self.info = Some(format!("Reemplazado en {n} {}", if n == 1 { "archivo" } else { "archivos" }));
        } else {
            self.notice = Some(stderr.lines().next().unwrap_or("No se pudo reemplazar").to_string());
        }
        self.search(project);
    }

    // ---------------------------------------------------------------- teclas

    fn rows_now(&self) -> Vec<Row> {
        self.results.rows(self.show_replace, &self.collapsed)
    }

    fn list_height(&self) -> usize {
        self.rows.saturating_sub(HEADER_ROWS + FOOTER_ROWS)
    }

    fn keep_visible(&mut self) {
        let height = self.list_height();
        if height == 0 {
            return;
        }
        if self.index < self.scroll {
            self.scroll = self.index;
        } else if self.index >= self.scroll + height {
            self.scroll = self.index + 1 - height;
        }
        self.scroll = self.scroll.min(self.rows_now().len().saturating_sub(height));
    }

    fn moved(&mut self, delta: isize) -> Request {
        let last = self.rows_now().len().saturating_sub(1) as isize;
        self.index = (self.index as isize + delta).clamp(0, last) as usize;
        Request::Redraw
    }

    fn toggle_file(&mut self, path: &str) {
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_string());
        }
    }

    pub fn key(&mut self, key: KeyWithModifier, project: &Path) -> Request {
        let plain = key.has_no_modifiers();
        let typed = plain || key.has_only_modifiers(&[KeyModifier::Shift]);
        let ctrl = key.has_only_modifiers(&[KeyModifier::Ctrl]);
        let rows = self.rows_now();
        let row = rows.get(self.index).copied();
        self.info = None;
        self.notice = None;
        let page = self.list_height().max(1) as isize;
        let request = match (key.bare_key, row) {
            // Opciones (VS Code: ⌥⌘C, ⌥⌘W, ⌥⌘R).
            (BareKey::Char('t'), _) if ctrl => {
                self.case_sensitive = !self.case_sensitive;
                self.search(project);
                Request::Redraw
            }
            (BareKey::Char('w'), _) if ctrl => {
                self.word = !self.word;
                self.search(project);
                Request::Redraw
            }
            (BareKey::Char('r'), _) if ctrl => {
                self.regex = !self.regex;
                self.search(project);
                Request::Redraw
            }
            (BareKey::Down, Some(Row::Query)) if plain && !self.show_replace => {
                self.moved(1)
            }
            (BareKey::Tab, Some(Row::Query)) if plain => {
                self.show_replace = true;
                self.index = 1;
                Request::Redraw
            }
            (BareKey::Tab | BareKey::Up, Some(Row::Replace)) if plain => {
                self.index = 0;
                Request::Redraw
            }
            (BareKey::Down, _) if plain => self.moved(1),
            (BareKey::Up, _) if plain => self.moved(-1),
            (BareKey::PageDown, _) if plain => self.moved(page),
            (BareKey::PageUp, _) if plain => self.moved(-page),
            (BareKey::End, _) if plain => self.moved(isize::MAX / 2),
            (BareKey::Home, Some(Row::File(_) | Row::Match(..))) if plain => {
                self.index = rows.iter().position(|r| matches!(r, Row::File(_))).unwrap_or(0);
                Request::Redraw
            }
            // Los campos.
            (BareKey::Enter, Some(Row::Query)) if plain => {
                self.search(project);
                Request::Redraw
            }
            (BareKey::Enter, Some(Row::Replace)) if plain => {
                self.ask_replace();
                Request::Redraw
            }
            (BareKey::Backspace, Some(field @ (Row::Query | Row::Replace))) => {
                self.field(field).pop();
                self.edited(field, project)
            }
            (BareKey::Char('u'), Some(field @ (Row::Query | Row::Replace))) if ctrl => {
                self.field(field).clear();
                self.edited(field, project)
            }
            (BareKey::Char(c), Some(field @ (Row::Query | Row::Replace))) if typed && !c.is_control() => {
                self.field(field).push(c);
                self.edited(field, project)
            }
            // Los resultados.
            (BareKey::Enter | BareKey::Right | BareKey::Left, Some(Row::File(f))) if plain => {
                let path = self.results.files[f].path.clone();
                let collapsed = self.collapsed.contains(&path);
                match key.bare_key {
                    BareKey::Right if !collapsed => {}
                    BareKey::Left if collapsed => {}
                    _ => self.toggle_file(&path),
                }
                Request::Redraw
            }
            (BareKey::Enter | BareKey::Right, Some(Row::Match(f, m))) if plain => self.open_match(f, m),
            (BareKey::Left, Some(Row::Match(f, _))) if plain => {
                self.index = rows.iter().position(|r| *r == Row::File(f)).unwrap_or(0);
                Request::Redraw
            }
            (BareKey::Esc, _) => Request::Redraw,
            _ => return Request::Ignored,
        };
        self.keep_visible();
        request
    }

    fn field(&mut self, row: Row) -> &mut String {
        if row == Row::Replace {
            &mut self.replacement
        } else {
            &mut self.query
        }
    }

    fn edited(&mut self, row: Row, project: &Path) -> Request {
        if row == Row::Query {
            self.search(project);
        }
        Request::Redraw
    }

    fn open_match(&self, f: usize, m: usize) -> Request {
        let file = &self.results.files[f];
        let hit = &file.matches[m];
        Request::Open(PathBuf::from(&file.path), hit.line, search::char_index(&hit.text, hit.col) + 1)
    }

    pub fn mouse(&mut self, mouse: Mouse) -> Request {
        match mouse {
            Mouse::LeftClick(line, _)
                if line >= HEADER_ROWS as isize && (line as usize) < self.rows.saturating_sub(FOOTER_ROWS) =>
            {
                let rows = self.rows_now();
                let index = line as usize - HEADER_ROWS + self.scroll;
                let Some(row) = rows.get(index).copied() else { return Request::Redraw };
                self.index = index;
                match row {
                    Row::File(f) => {
                        let path = self.results.files[f].path.clone();
                        self.toggle_file(&path);
                        Request::Redraw
                    }
                    Row::Match(f, m) => self.open_match(f, m),
                    _ => Request::Redraw,
                }
            }
            Mouse::ScrollUp(n) => {
                self.scroll = self.scroll.saturating_sub(n);
                Request::Redraw
            }
            Mouse::ScrollDown(n) => {
                let max = self.rows_now().len().saturating_sub(self.list_height());
                self.scroll = (self.scroll + n).min(max);
                Request::Redraw
            }
            _ => Request::Ignored,
        }
    }

    // ----------------------------------------------------------------- dibujo

    pub fn render(&mut self, rows: usize, cols: usize, focused: bool, p: &Palette, dim: Option<Look>) {
        self.rows = rows;
        let base = dim.unwrap_or(p.sidebar);
        let description = dim.map_or(p.description, |d| d.fg);

        // Cabecera: el título y las opciones activas resaltadas.
        let options = " Aa ab .* ";
        let title = fit(" BUSCAR", cols.saturating_sub(options.chars().count()));
        let pad = cols.saturating_sub(title.chars().count() + options.chars().count());
        let mut header = Line::new(&format!("{title}{}{options}", " ".repeat(pad)), cols, dim.unwrap_or(p.header).bold());
        let start = cols.saturating_sub(options.chars().count());
        for (on, from, to) in [(self.case_sensitive, 1, 3), (self.word, 4, 6), (self.regex, 7, 9)] {
            if on && start + to <= cols {
                header.fg(start + from..start + to, p.accent);
            }
        }
        print!("{}", header.render(0, 0));

        self.keep_visible();
        let selection = if focused { p.selection } else { p.inactive_selection };
        let list = self.rows_now();
        for y in 0..self.list_height() {
            let i = self.scroll + y;
            let line = match list.get(i) {
                Some(row) => self.row_line(*row, i == self.index, focused, cols, p, base, description, selection),
                None => Line::new("", cols, base),
            };
            print!("{}", line.render(0, HEADER_ROWS + y));
        }

        let (text, color) = match (&self.notice, &self.info) {
            (Some(notice), _) => (notice.clone(), p.error),
            (None, Some(info)) => (info.clone(), p.accent),
            _ => (self.footer(), description),
        };
        let mut footer = Line::new(&format!(" {text}"), cols, base);
        footer.fg(0..cols, color);
        print!("{}", footer.render(0, rows.saturating_sub(1)));
    }

    fn footer(&self) -> String {
        if self.running {
            return "Buscando…".to_string();
        }
        if !self.searched {
            return "Escribe para buscar · Tab reemplazar".to_string();
        }
        let (n, f) = (self.results.total, self.results.files.len());
        if n == 0 {
            return "Sin resultados".to_string();
        }
        let more = if self.results.truncated { "+" } else { "" };
        format!("{n}{more} en {f} {}", if f == 1 { "archivo" } else { "archivos" })
    }

    #[allow(clippy::too_many_arguments)]
    fn row_line(&self, row: Row, selected: bool, focused: bool, cols: usize, p: &Palette, base: Look, description: Rgb, selection: Look) -> Line {
        match row {
            Row::Query | Row::Replace => {
                let (text, placeholder, mark) = if row == Row::Query {
                    (&self.query, "Buscar", " › ")
                } else {
                    (&self.replacement, "Reemplazar (Enter: todo)", " ⇄ ")
                };
                let cursor = if selected && focused { "▏" } else { "" };
                let room = cols.saturating_sub(5);
                let count = text.chars().count();
                let shown: String = if count > room {
                    format!("…{}", text.chars().skip(count + 1 - room).collect::<String>())
                } else {
                    text.clone()
                };
                let body = if text.is_empty() { format!("{cursor}{placeholder}") } else { format!("{shown}{cursor}") };
                let mut line = Line::new(&format!("{mark}{body}"), cols, base);
                line.fg(1..2, p.accent);
                if text.is_empty() {
                    line.fg(3 + cursor.chars().count()..cols, description);
                }
                line
            }
            Row::File(f) => {
                let file = &self.results.files[f];
                let (name, dir) = file.name_and_dir();
                let icon = if self.collapsed.contains(&file.path) { "▸" } else { "▾" };
                let count = file.matches.len().to_string();
                let left = fit(&format!(" {icon} {name}  {dir}"), cols.saturating_sub(count.chars().count() + 2));
                let mut line = Line::new(&format!("{left} {count} "), cols, base);
                if selected {
                    line.paint(0..cols, |l| Look { bg: selection.bg, fg: selection.fg, ..l });
                }
                let name_end = 3 + name.chars().count();
                line.fg(name_end..cols.saturating_sub(count.chars().count() + 2), description);
                line
            }
            Row::Match(f, m) => {
                let hit = &self.results.files[f].matches[m];
                let (text, removed) = search::trimmed(&hit.text);
                let mut line = Line::new(&format!("     {text}"), cols, base);
                if !self.regex {
                    let start = 5 + search::char_index(&hit.text, hit.col).saturating_sub(removed);
                    let end = (start + self.query.chars().count()).min(cols);
                    if start < end {
                        line.fg(start..end, p.accent);
                    }
                }
                if selected {
                    line.paint(0..cols, |l| Look { bg: selection.bg, fg: l.fg, ..l });
                }
                line
            }
        }
    }
}
