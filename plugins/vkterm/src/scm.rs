//! Vista «Control de código fuente» (⌃⇧G), como "Source Control" de VS Code,
//! en la misma barra lateral que el explorador: ⌃⇧G y ⌘⇧E cambian de vista
//! (src/main.rs).
//!
//! - Cabecera: la rama, los commits por subir y por bajar y la fusión (o
//!   rebase…) en curso.
//! - El mensaje del commit: Enter o ⌘Enter hace commit. Sin nada preparado,
//!   pregunta si preparar todo, como VS Code.
//! - La comparación con otra rama (main, si no se elige otra): si fusionarla
//!   provocaría conflictos, y en qué archivos (`git merge-tree`, sin tocar
//!   nada). Enter elige otra rama (src/branches.rs).
//! - Grupos plegables: «Conflictos de fusión», «Cambios preparados»,
//!   «Cambios» y los conflictos con la otra rama. Espacio prepara o quita,
//!   Enter muestra las diferencias en una ventana flotante (el paginador de
//!   git), → abre el archivo y ⌫ descarta los cambios (pregunta antes).
//!
//! El estado lo lee `bin/vkterm-git status` (interpretado en src/git.rs) al
//! cambiar algo en el disco, cada pocos segundos y tras cada acción. Pull,
//! Push y Sincronizar se ejecutan desde la paleta en una ventana flotante,
//! donde se ve la salida y se pueden escribir credenciales.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use vkterm::git::{Change, Check, Decorations, Group, Kind, Row, ScmList, Status};
use vkterm::question::Question;
use vkterm::style::{fit, Line, Look, Palette, Rgb};
use vkterm::themes::GitColors;
use zellij_tile::prelude::*;

pub const TITLE: &str = "Control de código fuente";
const HEADER_ROWS: usize = 1;
const FOOTER_ROWS: usize = 1;

/// Lo que la vista pide al controlador.
pub enum Request {
    /// No era una tecla de la vista.
    Ignored,
    Redraw,
    /// Abrir en el editor (relativa al proyecto, o absoluta si está fuera).
    Open(PathBuf),
}

/// Acción que espera la respuesta de la ventana de confirmación.
enum Pending {
    Discard(Vec<String>),
    Clean(Vec<String>),
    DiscardAll { tracked: Vec<String>, untracked: Vec<String> },
    CommitAll,
    Stage(Vec<String>),
    Abort,
}

#[derive(Default)]
pub struct Scm {
    pub status: Status,
    loaded: bool,
    list: ScmList,
    message: String,
    /// El mensaje lo ha escrito el usuario (no se sustituye por el de git).
    edited: bool,
    pub decorations: Decorations,
    /// Rama con la que se comparan los conflictos ("" = la principal).
    pub target: String,
    /// `status` en marcha, y otro pendiente cuando termine.
    running: bool,
    dirty: bool,
    /// Volver a comparar las ramas aunque no hayan cambiado los commits.
    recheck: bool,
    pending: Option<Pending>,
    /// Ventanas flotantes abiertas: (panel, tipo).
    windows: Vec<(u32, String)>,
    /// Aviso en el pie (errores de git).
    pub notice: Option<String>,
    pub colors: GitColors,
    rows: usize,
}

impl Scm {
    // ------------------------------------------------------------- estado

    /// Vuelve a leer el estado en la próxima ocasión (el tick del controlador).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn tick(&mut self, project: &Path) {
        if self.dirty && !self.running {
            self.refresh(project);
        }
    }

    pub fn refresh(&mut self, project: &Path) {
        if self.running {
            self.dirty = true;
            return;
        }
        self.running = true;
        self.dirty = false;
        let c = &self.status.compare;
        // Con el resultado de la vez anterior, git no repite la comparación si
        // no han cambiado los commits.
        let last = match c.check {
            Check::Clean | Check::Conflicts | Check::Unrelated if !self.recheck => c.key.clone(),
            _ => String::new(),
        };
        self.recheck = false;
        run_command_with_env_variables_and_cwd(
            &["vkterm-git", "status", &self.target, &last],
            BTreeMap::new(),
            project.to_path_buf(),
            crate::context("git_status"),
        );
    }

    /// El resultado de `vkterm-git status`.
    pub fn status_result(&mut self, code: Option<i32>, stdout: &str, project: &Path) {
        self.running = false;
        if code == Some(0) {
            let mut status = Status::parse(stdout);
            status.compare.keep_from(&self.status.compare);
            // Otra rama en los mismos commits: hay que compararla de verdad.
            if status.compare.check == Check::Same {
                self.recheck = true;
                self.dirty = true;
            }
            // Como VS Code: durante una fusión, el mensaje que propone git.
            if !self.edited && status.message_suggestion() != self.status.message_suggestion() {
                self.message = status.message_suggestion().to_string();
            }
            self.decorations = Decorations::new(&status);
            self.status = status;
            self.loaded = true;
            self.list.update(&self.status);
        } else if !self.loaded {
            // Sin bin/vkterm-git (o sin git): no se queda en «Leyendo…».
            self.loaded = true;
            self.notice = Some("No se pudo leer el estado de git".to_string());
        }
        if self.dirty {
            self.refresh(project);
        }
    }

    /// El resultado de una acción (preparar, descartar, commit…).
    pub fn action_result(&mut self, kind: &str, code: Option<i32>, stdout: &str, stderr: &str, project: &Path) {
        let output = format!("{stderr}{stdout}");
        if code == Some(0) {
            self.notice = None;
            if kind == "git_commit" {
                self.message.clear();
                self.edited = false;
            }
        } else {
            let first = output.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("git falló");
            self.notice = Some(first.to_string());
            // La salida completa (un hook que rechaza el commit), en una ventana.
            if kind == "git_commit" && output.trim().lines().count() > 1 {
                self.open_window(&["show", output.trim()], "Commit: salida de git", "show");
            }
        }
        self.refresh(project);
    }

    /// Una ventana flotante de git (diferencias, pull…) terminó: se cierra.
    pub fn window_exited(&mut self, id: u32, code: Option<i32>, project: &Path) -> bool {
        let Some(i) = self.windows.iter().position(|(w, _)| *w == id) else { return false };
        let (_, kind) = self.windows.remove(i);
        close_terminal_pane(id);
        if kind == "remote" && code.is_some_and(|c| c != 0) {
            self.notice = Some("git falló: la salida estaba en su ventana".to_string());
        }
        self.refresh(project);
        true
    }

    /// Rama y avisos para la barra de estado: ("rama* ↓0 ↑1", "⚠ …").
    pub fn bar(&self) -> (String, String) {
        let s = &self.status;
        if !s.repo || s.branch.is_empty() {
            return (String::new(), String::new());
        }
        let mut branch = s.branch.clone();
        if s.dirty() {
            branch.push('*');
        }
        if s.upstream.is_some() {
            branch += &format!("  ↓{} ↑{}", s.behind, s.ahead);
        }
        let notice = if !s.merge.is_empty() {
            format!("⚠ {}: {}", operation_name(&s.operation), conflicts(s.merge.len()))
        } else if s.compare.check == Check::Conflicts {
            format!("⚠ {} con {}", conflicts(s.compare.files.len()), s.compare.target)
        } else {
            String::new()
        };
        (branch, notice)
    }

    // ---------------------------------------------------------- acciones

    fn root(&self) -> Option<PathBuf> {
        (self.status.repo && !self.status.root.is_empty()).then(|| PathBuf::from(&self.status.root))
    }

    /// `vkterm-git ARGS…` en segundo plano, desde la raíz del repositorio.
    fn run(&mut self, args: &[&str], kind: &str) {
        let Some(root) = self.root() else { return };
        let mut command = vec!["vkterm-git"];
        command.extend_from_slice(args);
        run_command_with_env_variables_and_cwd(&command, BTreeMap::new(), root, crate::context(kind));
    }

    fn run_paths(&mut self, action: &str, paths: &[String]) {
        if paths.is_empty() {
            return;
        }
        let mut args = vec![action];
        args.extend(paths.iter().map(String::as_str));
        self.run(&args, "git_action");
    }

    /// `vkterm-git ARGS…` en una ventana flotante con el foco, que se cierra
    /// cuando termina.
    fn open_window(&mut self, args: &[&str], title: &str, kind: &str) {
        let Some(root) = self.root() else { return };
        let command = CommandToRun {
            path: PathBuf::from("vkterm-git"),
            args: args.iter().map(|a| a.to_string()).collect(),
            cwd: Some(root),
        };
        let size = |n: &str| Some(n.to_string());
        let coordinates = FloatingPaneCoordinates::new(size("10%"), size("8%"), size("80%"), size("84%"), None, None);
        if let Some(PaneId::Terminal(id)) = open_command_pane_floating(command, coordinates, crate::context("git_window")) {
            rename_terminal_pane(id, title);
            self.windows.push((id, kind.to_string()));
        }
    }

    /// Desde la paleta: pull, push, sync, fetch, stage_all, unstage_all,
    /// commit, compare, abort, refresh.
    pub fn command(&mut self, name: &str, project: &Path) {
        if !self.status.repo {
            self.notice = Some("La carpeta no tiene un repositorio git".to_string());
            return;
        }
        match name {
            "pull" | "push" | "sync" | "fetch" => self.open_window(&["remote", name], &format!("git {name}"), "remote"),
            "stage_all" => self.run(&["stage-all"], "git_action"),
            "unstage_all" => self.run(&["unstage-all"], "git_action"),
            "commit" => self.commit(),
            "compare" => self.pick_branch(),
            "abort" if self.status.operation.is_empty() => {
                self.notice = Some("No hay ninguna fusión en curso".to_string());
            }
            "abort" => {
                let what = operation_name(&self.status.operation);
                let lines = [
                    format!("Se abandonará la {what} en curso y lo resuelto hasta ahora:"),
                    "los archivos vuelven a como estaban antes de empezarla.".to_string(),
                ];
                self.ask(Pending::Abort, Question::confirm(&format!("Abortar la {what}"), &lines, "Abortar", "scm_answer", "yes"));
            }
            "refresh" => self.refresh(project),
            _ => {}
        }
    }

    fn ask(&mut self, pending: Pending, question: Question) {
        self.pending = Some(pending);
        crate::launch_window("ask", "confirm", Some(question.to_text()));
    }

    /// La respuesta de la ventana de confirmación.
    pub fn answer(&mut self) {
        match self.pending.take() {
            Some(Pending::Discard(paths)) => self.run_paths("discard", &paths),
            Some(Pending::Clean(paths)) => self.run_paths("clean", &paths),
            Some(Pending::DiscardAll { tracked, untracked }) => {
                self.run_paths("discard", &tracked);
                self.run_paths("clean", &untracked);
            }
            Some(Pending::CommitAll) => self.run_commit(true),
            Some(Pending::Stage(paths)) => self.run_paths("stage", &paths),
            Some(Pending::Abort) => self.run(&["abort"], "git_action"),
            None => {}
        }
    }

    /// Enter en el mensaje, ⌘Enter o «Git: Commit».
    pub fn commit(&mut self) {
        let s = &self.status;
        if !s.repo {
            return;
        }
        if self.message.trim().is_empty() {
            self.notice = Some("Escribe el mensaje del commit".to_string());
            self.list.select_message();
            return;
        }
        if !s.staged.is_empty() || (s.operation == "merge" && s.merge.is_empty()) {
            self.run_commit(false);
        } else if !s.merge.is_empty() {
            self.notice = Some("Quedan conflictos: prepara (Espacio) los ya resueltos".to_string());
        } else if s.changes.is_empty() {
            self.notice = Some("No hay cambios para hacer commit".to_string());
        } else {
            let lines = ["No hay cambios preparados.".to_string(), "¿Preparar todos los cambios y hacer commit?".to_string()];
            self.ask(Pending::CommitAll, Question::confirm("Commit", &lines, "Preparar todo y hacer commit", "scm_answer", "yes"));
        }
    }

    fn run_commit(&mut self, all: bool) {
        let message = self.message.trim().to_string();
        let mut args = vec!["commit"];
        if all {
            args.push("--all");
        }
        args.push(&message);
        self.run(&args, "git_commit");
    }

    pub fn pick_branch(&mut self) {
        let current = if self.target.is_empty() { String::new() } else { self.status.compare.target.clone() };
        crate::launch_window("toggle_branches", "branches", Some(current));
    }

    /// Espacio: preparar o quitar (un archivo o todo su grupo).
    fn toggle_stage(&mut self, group: Group, changes: &[Change]) {
        let paths: Vec<String> = changes.iter().flat_map(|c| c.orig.iter().chain([&c.path])).cloned().collect();
        match group {
            Group::Staged => self.run_paths("unstage", &paths),
            Group::Changes => self.run_paths("stage", &paths),
            // Marcar como resuelto: si aún tiene marcas de conflicto, pregunta.
            Group::Merge => {
                let marked: Vec<&str> = changes.iter().filter(|c| self.has_markers(&c.path)).map(|c| c.path.as_str()).collect();
                if marked.is_empty() {
                    self.run_paths("stage", &paths);
                } else {
                    let lines = [
                        format!("{} todavía tiene marcas de conflicto (<<<<<<<).", short_list(&marked)),
                        "¿Prepararlo de todos modos?".to_string(),
                    ];
                    self.ask(Pending::Stage(paths), Question::confirm("Marcar como resuelto", &lines, "Preparar", "scm_answer", "yes"));
                }
            }
            Group::Incoming => {}
        }
    }

    /// Si el archivo (dentro del proyecto) tiene marcas de conflicto.
    fn has_markers(&self, path: &str) -> bool {
        let Some(relative) = path.strip_prefix(self.status.prefix.as_str()) else { return false };
        std::fs::read_to_string(Path::new(crate::HOST).join(relative))
            .map(|text| text.lines().any(|l| l.starts_with("<<<<<<< ") || l.starts_with(">>>>>>> ")))
            .unwrap_or(false)
    }

    /// ⌫: descartar los cambios (y borrar los archivos nuevos), preguntando.
    fn discard(&mut self, changes: &[Change]) {
        let (untracked, tracked): (Vec<&Change>, Vec<&Change>) = changes.iter().partition(|c| c.kind == Kind::Untracked);
        let paths = |list: &[&Change]| list.iter().map(|c| c.path.clone()).collect::<Vec<_>>();
        let (tracked, untracked) = (paths(&tracked), paths(&untracked));
        let undo = "No se puede deshacer.".to_string();
        match (tracked.len(), untracked.len()) {
            (0, 0) => {}
            (1, 0) => {
                let lines = [format!("¿Descartar los cambios de {}?", name(&tracked[0])), undo];
                self.ask(Pending::Discard(tracked), Question::confirm("Descartar cambios", &lines, "Descartar cambios", "scm_answer", "yes"));
            }
            (0, 1) => {
                let lines = [format!("{} es un archivo nuevo: se borrará.", name(&untracked[0])), undo];
                self.ask(Pending::Clean(untracked), Question::confirm("Borrar archivo", &lines, "Borrar archivo", "scm_answer", "yes"));
            }
            (t, u) => {
                let mut lines = Vec::new();
                if t > 0 {
                    lines.push(format!("Se descartarán los cambios de {t} {}.", plural(t, "archivo", "archivos")));
                }
                if u > 0 {
                    lines.push(format!("Se borrarán {u} {}.", plural(u, "archivo nuevo", "archivos nuevos")));
                }
                lines.push(undo);
                self.ask(
                    Pending::DiscardAll { tracked, untracked },
                    Question::confirm("Descartar todos los cambios", &lines, "Descartar todo", "scm_answer", "yes"),
                );
            }
        }
    }

    /// Enter en un archivo: sus diferencias en una ventana flotante.
    fn show_diff(&mut self, group: Group, change: &Change) {
        let (mode, title) = match group {
            Group::Staged => ("staged".to_string(), format!("Diferencias (preparado): {}", change.path)),
            Group::Changes if change.kind == Kind::Untracked => ("untracked".to_string(), format!("Archivo nuevo: {}", change.path)),
            Group::Changes => ("changes".to_string(), format!("Diferencias: {}", change.path)),
            Group::Merge => ("merge".to_string(), format!("Conflicto: {}", change.path)),
            Group::Incoming => {
                let c = &self.status.compare;
                (c.tree.clone(), format!("Conflicto al fusionar {}: {}", c.target, change.path))
            }
        };
        let mut args = vec!["diff", mode.as_str(), change.path.as_str()];
        if let (Group::Staged, Some(orig)) = (group, &change.orig) {
            args.push(orig);
        }
        self.open_window(&args, &title, "diff");
    }

    /// →: el archivo en el editor.
    fn open_file(&self, change: &Change) -> Request {
        if change.kind == Kind::Deleted {
            return Request::Redraw;
        }
        match change.path.strip_prefix(self.status.prefix.as_str()) {
            Some(relative) => Request::Open(PathBuf::from(relative)),
            None => Request::Open(Path::new(&self.status.root).join(&change.path)),
        }
    }

    // ------------------------------------------------------ teclado y ratón

    fn list_height(&self) -> usize {
        self.rows.saturating_sub(HEADER_ROWS + FOOTER_ROWS)
    }

    /// Al mostrar la vista con ⌃⇧G, el foco va al mensaje, como en VS Code.
    pub fn focus_message(&mut self) {
        self.list.select_message();
    }

    pub fn key(&mut self, key: KeyWithModifier) -> Request {
        if !self.status.repo {
            return Request::Ignored;
        }
        let plain = key.has_no_modifiers();
        let typed = plain || key.has_only_modifiers(&[KeyModifier::Shift]);
        let page = self.list_height().max(1) as isize;
        let row = self.list.selected().cloned();
        // Cada tecla borra el aviso anterior; el commit puede poner otro.
        let notice = self.notice.take();
        let request = match (key.bare_key, row) {
            (BareKey::Down, _) if plain => self.moved(1),
            (BareKey::Up, _) if plain => self.moved(-1),
            (BareKey::PageDown, _) if plain => self.moved(page),
            (BareKey::PageUp, _) if plain => self.moved(-page),
            // El mensaje del commit.
            (BareKey::Enter, Some(Row::Message)) if plain => {
                self.commit();
                Request::Redraw
            }
            (BareKey::Backspace, Some(Row::Message)) => {
                self.message.pop();
                self.edited = true;
                Request::Redraw
            }
            (BareKey::Char('u'), Some(Row::Message)) if key.has_only_modifiers(&[KeyModifier::Ctrl]) => {
                self.message.clear();
                self.edited = true;
                Request::Redraw
            }
            (BareKey::Char(c), Some(Row::Message)) if typed && !c.is_control() => {
                self.message.push(c);
                self.edited = true;
                Request::Redraw
            }
            (BareKey::Home, Some(row)) if plain && row != Row::Message => {
                self.list.select(1);
                Request::Redraw
            }
            (BareKey::End, _) if plain => self.moved(isize::MAX / 2),
            (BareKey::Enter, Some(Row::Compare)) if plain => {
                self.pick_branch();
                Request::Redraw
            }
            // La cabecera de un grupo.
            (BareKey::Enter, Some(Row::Header { group, expanded, .. })) if plain => {
                self.list.set_expanded(&self.status, group, !expanded);
                Request::Redraw
            }
            (BareKey::Left, Some(Row::Header { group, .. })) if plain => {
                self.list.set_expanded(&self.status, group, false);
                Request::Redraw
            }
            (BareKey::Right, Some(Row::Header { group, .. })) if plain => {
                self.list.set_expanded(&self.status, group, true);
                Request::Redraw
            }
            (BareKey::Char(' '), Some(Row::Header { group, .. })) if plain => {
                match group {
                    Group::Staged => self.run(&["unstage-all"], "git_action"),
                    _ => self.toggle_stage(group, &self.status.group(group)),
                }
                Request::Redraw
            }
            (BareKey::Backspace | BareKey::Delete, Some(Row::Header { group: Group::Changes, .. })) => {
                self.discard(&self.status.changes.clone());
                Request::Redraw
            }
            // Un archivo.
            (BareKey::Enter, Some(Row::File { group, change })) if plain => {
                self.show_diff(group, &change);
                Request::Redraw
            }
            (BareKey::Char(' '), Some(Row::File { group, change })) if plain => {
                self.toggle_stage(group, &[change]);
                Request::Redraw
            }
            (BareKey::Right, Some(Row::File { change, .. })) if plain => self.open_file(&change),
            (BareKey::Left, Some(Row::File { group, .. })) if plain => {
                self.list.set_expanded(&self.status, group, false);
                Request::Redraw
            }
            (BareKey::Backspace | BareKey::Delete, Some(Row::File { group: Group::Changes, change })) => {
                self.discard(&[change]);
                Request::Redraw
            }
            (BareKey::Esc, _) => Request::Redraw,
            _ => {
                self.notice = notice;
                return Request::Ignored;
            }
        };
        self.list.keep_visible(self.list_height());
        request
    }

    fn moved(&mut self, delta: isize) -> Request {
        self.list.move_by(delta);
        Request::Redraw
    }

    pub fn mouse(&mut self, mouse: Mouse) -> Request {
        match mouse {
            Mouse::LeftClick(line, _)
                if line >= HEADER_ROWS as isize
                    && (line as usize) < self.rows.saturating_sub(FOOTER_ROWS)
                    && self.status.repo =>
            {
                let index = line as usize - HEADER_ROWS + self.list.scroll;
                if index >= self.list.rows().len() {
                    return Request::Redraw;
                }
                self.list.select(index);
                match self.list.selected().cloned() {
                    Some(Row::Compare) => self.pick_branch(),
                    Some(Row::Header { group, expanded, .. }) => self.list.set_expanded(&self.status, group, !expanded),
                    Some(Row::File { group, change }) => self.show_diff(group, &change),
                    _ => {}
                }
                Request::Redraw
            }
            Mouse::ScrollUp(n) => {
                self.list.scroll_by(-(n as isize), self.list_height());
                Request::Redraw
            }
            Mouse::ScrollDown(n) => {
                self.list.scroll_by(n as isize, self.list_height());
                Request::Redraw
            }
            _ => Request::Ignored,
        }
    }

    // ------------------------------------------------------------- dibujo

    pub fn render(&mut self, rows: usize, cols: usize, focused: bool, p: &Palette, dim: Option<Look>) {
        self.rows = rows;
        let base = dim.unwrap_or(p.sidebar);
        let description = dim.map_or(p.description, |d| d.fg);
        let s = &self.status;
        let mut header = format!(" ⎇ {}", s.branch);
        if s.upstream.is_some() {
            header += &format!("  ↓{} ↑{}", s.behind, s.ahead);
        }
        if !s.operation.is_empty() {
            header += &format!("  · {} en curso", operation_name(&s.operation));
        }
        if !s.repo {
            header = format!(" {}", TITLE.to_uppercase());
        }
        print!("{}", Line::new(&header, cols, dim.unwrap_or(p.header).bold()).render(0, 0));

        let height = self.list_height();
        let empty = |text: &str| {
            let mut line = Line::new(text, cols, base);
            line.fg(0..cols, description);
            line
        };
        if !self.loaded || !s.repo {
            let lines = if !self.loaded {
                vec![" Leyendo el estado de git…"]
            } else {
                vec![" La carpeta no tiene un repositorio git.", "", " Para crearlo: git init en el Shell (⌘J)."]
            };
            for y in 0..height {
                print!("{}", empty(lines.get(y).copied().unwrap_or("")).render(0, HEADER_ROWS + y));
            }
        } else {
            self.list.keep_visible(height);
            for y in 0..height {
                let i = self.list.scroll + y;
                let line = match self.list.rows().get(i) {
                    Some(row) => self.row_line(row, i == self.list.index(), focused, cols, p, base, description),
                    None => Line::new("", cols, base),
                };
                print!("{}", line.render(0, HEADER_ROWS + y));
            }
        }

        let (text, color) = match &self.notice {
            Some(notice) => (notice.as_str(), p.error),
            None => (self.hint(), description),
        };
        let mut footer = Line::new(&format!(" {text}"), cols, base);
        footer.fg(0..cols, color);
        print!("{}", footer.render(0, rows.saturating_sub(1)));
    }

    #[allow(clippy::too_many_arguments)]
    fn row_line(&self, row: &Row, selected: bool, focused: bool, cols: usize, p: &Palette, base: Look, description: Rgb) -> Line {
        let s = &self.status;
        let selection = if focused { p.selection } else { p.inactive_selection };
        let mut line;
        match row {
            Row::Message => {
                let cursor = selected && focused;
                let room = cols.saturating_sub(5);
                let count = self.message.chars().count();
                // Si no cabe, se ve el final (donde se escribe).
                let shown: String = if count > room {
                    let tail: String = self.message.chars().skip(count + 1 - room).collect();
                    format!("…{tail}")
                } else {
                    self.message.clone()
                };
                let mark = if cursor { "▏" } else { "" };
                let text = if self.message.is_empty() {
                    format!(" › {mark}Mensaje (Enter: commit en «{}»)", s.branch)
                } else {
                    format!(" › {shown}{mark}")
                };
                line = Line::new(&text, cols, base);
                line.fg(1..2, p.accent);
                if self.message.is_empty() {
                    line.fg(3 + mark.chars().count()..cols, description);
                }
            }
            Row::Compare => {
                let c = &s.compare;
                let sync = if c.ahead + c.behind > 0 { format!(" ↓{} ↑{}", c.behind, c.ahead) } else { String::new() };
                let text = match c.check {
                    Check::None => " ⇄ Comparar con otra rama…".to_string(),
                    Check::Same => format!(" ⇄ {}: comprobando…", c.target),
                    Check::Clean => format!(" ⇄ {}{sync} · sin conflictos", c.target),
                    Check::Conflicts => format!(" ⚠ {}{sync} · {}", c.target, conflicts(c.files.len())),
                    Check::Unrelated => format!(" ⇄ {}: sin historia común", c.target),
                    Check::Missing => format!(" ⇄ {}: la rama ya no existe", c.target),
                    Check::Unsupported => " ⇄ Comparar ramas pide git 2.38 o más nuevo".to_string(),
                    Check::Error => format!(" ⇄ {}: no se pudo comprobar", c.target),
                };
                line = Line::new(&text, cols, base);
                if selected {
                    line.paint(0..cols, |_| selection);
                }
                let color = if c.check == Check::Conflicts { self.colors.conflicting } else { description };
                line.fg(0..cols, color);
                return line;
            }
            Row::Header { group, count, expanded } => {
                let icon = if *expanded { "▾" } else { "▸" };
                let title = match group {
                    Group::Merge => "CONFLICTOS DE FUSIÓN".to_string(),
                    Group::Staged => "CAMBIOS PREPARADOS".to_string(),
                    Group::Changes => "CAMBIOS".to_string(),
                    Group::Incoming => format!("CONFLICTOS CON {}", s.compare.target),
                };
                let count = count.to_string();
                let left = fit(&format!(" {icon} {title}"), cols.saturating_sub(count.chars().count() + 2));
                line = Line::new(&format!("{left} {count} "), cols, base.bold());
            }
            Row::File { group, change } => {
                let (name, dir) = change.name_and_dir();
                let letter = change.kind.letter();
                let left = fit(&format!("     {name}  {dir}"), cols.saturating_sub(3));
                line = Line::new(&format!("{left} {letter} "), cols, base);
                if selected {
                    line.paint(0..cols, |_| selection);
                }
                let color = self.color(*group, change.kind);
                let name_end = 5 + name.chars().count();
                line.fg(5..name_end.min(cols.saturating_sub(3)), color);
                line.fg(name_end..cols.saturating_sub(3), description);
                line.fg(cols.saturating_sub(2)..cols.saturating_sub(1), color);
                return line;
            }
        }
        if selected && !matches!(row, Row::Message) {
            line.paint(0..cols, |l| Look { bg: selection.bg, fg: selection.fg, ..l });
        }
        line
    }

    /// Color de un archivo según su estado (gitDecoration.* del tema).
    pub fn color(&self, group: Group, kind: Kind) -> Rgb {
        if group == Group::Incoming {
            return self.colors.conflicting;
        }
        kind_color(&self.colors, kind)
    }

    fn hint(&self) -> &'static str {
        match self.list.selected() {
            _ if !self.status.repo => "⌘⇧E explorador · ⌘B ocultar",
            Some(Row::Message) => "Enter commit · ↓ cambios",
            Some(Row::Compare) => "Enter elegir otra rama",
            Some(Row::Header { group: Group::Staged, .. }) => "Espacio quitar todo · Enter plegar",
            Some(Row::Header { group: Group::Changes, .. }) => "Espacio preparar todo · ⌫ descartar",
            Some(Row::Header { group: Group::Merge, .. }) => "Espacio marcar todo resuelto",
            Some(Row::Header { .. }) => "Enter plegar o desplegar",
            Some(Row::File { group: Group::Staged, .. }) => "Espacio quitar · Enter dif. · → abrir",
            Some(Row::File { group: Group::Changes, .. }) => "Espacio preparar · Enter dif. · ⌫ descartar",
            Some(Row::File { group: Group::Merge, .. }) => "Espacio resuelto · Enter conflicto · → abrir",
            Some(Row::File { group: Group::Incoming, .. }) => "Enter ver el conflicto · → abrir",
            None => "",
        }
    }
}

/// Color de un estado de git en el tema.
pub fn kind_color(colors: &GitColors, kind: Kind) -> Rgb {
    match kind {
        Kind::Modified | Kind::TypeChanged => colors.modified,
        Kind::Added | Kind::Copied => colors.added,
        Kind::Deleted => colors.deleted,
        Kind::Renamed => colors.renamed,
        Kind::Untracked => colors.untracked,
        Kind::Conflict => colors.conflicting,
    }
}

fn operation_name(operation: &str) -> &'static str {
    match operation {
        "rebase" => "rebase",
        "cherry-pick" => "cherry-pick",
        "revert" => "reversión",
        _ => "fusión",
    }
}

fn conflicts(n: usize) -> String {
    format!("{n} {}", plural(n, "conflicto", "conflictos"))
}

fn plural(n: usize, one: &'static str, many: &'static str) -> &'static str {
    if n == 1 {
        one
    } else {
        many
    }
}

fn name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// "a.rs", "a.rs y b.rs" o "a.rs y 3 más".
fn short_list(paths: &[&str]) -> String {
    match paths {
        [one] => name(one).to_string(),
        [a, b] => format!("{} y {}", name(a), name(b)),
        [a, rest @ ..] => format!("{} y {} más", name(a), rest.len()),
        [] => String::new(),
    }
}
