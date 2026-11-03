//! Estado de git para la vista «Control de código fuente» (⌃⇧G) y para los
//! colores del explorador, como la extensión git de VS Code.
//!
//! Interpreta la salida de `bin/vkterm-git status`: registros separados por
//! NUL, primero clave=valor (raíz, operación en curso y la comprobación de
//! conflictos con otra rama) y, tras un "--", los de
//! `git status --porcelain=v2 -z --branch`.
//!
//! Las rutas de git son relativas a la raíz del repositorio; las del
//! explorador, a la carpeta del proyecto (que puede estar dentro de él:
//! `prefix`).
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Estado de un archivo, con la letra de VS Code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Conflict,
}

impl Kind {
    pub fn letter(self) -> char {
        match self {
            Kind::Modified => 'M',
            Kind::Added => 'A',
            Kind::Deleted => 'D',
            Kind::Renamed => 'R',
            Kind::Copied => 'C',
            Kind::TypeChanged => 'T',
            Kind::Untracked => 'U',
            Kind::Conflict => '!',
        }
    }

    /// Qué color manda en una carpeta con cambios de varios tipos.
    fn weight(self) -> u8 {
        match self {
            Kind::Conflict => 4,
            Kind::Modified | Kind::TypeChanged => 3,
            Kind::Deleted => 2,
            Kind::Added | Kind::Renamed | Kind::Copied | Kind::Untracked => 1,
        }
    }

    /// Una letra de `git status` (X o Y).
    fn from_code(code: char) -> Option<Kind> {
        Some(match code {
            'M' => Kind::Modified,
            'A' => Kind::Added,
            'D' => Kind::Deleted,
            'R' => Kind::Renamed,
            'C' => Kind::Copied,
            'T' => Kind::TypeChanged,
            _ => return None,
        })
    }
}

/// Los grupos de la vista, en su orden.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    /// Conflictos de una fusión (o rebase…) en curso.
    Merge,
    Staged,
    Changes,
    /// Archivos que chocarían al fusionar la rama de comparación.
    Incoming,
}

pub const GROUPS: [Group; 4] = [Group::Merge, Group::Staged, Group::Changes, Group::Incoming];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    /// Relativa a la raíz del repositorio.
    pub path: String,
    /// La ruta anterior de un archivo renombrado o copiado.
    pub orig: Option<String>,
    pub kind: Kind,
}

impl Change {
    /// Nombre del archivo y su carpeta, como en la lista de VS Code.
    pub fn name_and_dir(&self) -> (&str, &str) {
        match self.path.rsplit_once('/') {
            Some((dir, name)) => (name, dir),
            None => (&self.path, ""),
        }
    }
}

/// La comprobación de conflictos con otra rama (`git merge-tree`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Check {
    /// Sin rama con la que comparar, o es la actual, o no hay commits.
    #[default]
    None,
    /// Los mismos commits que la vez anterior: vale el resultado anterior.
    Same,
    Clean,
    Conflicts,
    /// Sin historia en común.
    Unrelated,
    /// La rama ya no existe.
    Missing,
    /// git < 2.38 no tiene `merge-tree --write-tree`.
    Unsupported,
    Error,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compare {
    pub target: String,
    /// "HEAD:RAMA" (los commits comparados), para no repetir el cálculo.
    pub key: String,
    pub check: Check,
    /// Árbol con el resultado y sus marcas de conflicto (para verlas).
    pub tree: String,
    pub files: Vec<String>,
    /// Commits de HEAD que no tiene la rama, y al revés.
    pub ahead: u32,
    pub behind: u32,
}

impl Compare {
    /// Con `Same`, el resultado de la vez anterior (si era de la misma rama).
    pub fn keep_from(&mut self, previous: &Compare) {
        if self.check == Check::Same && previous.target == self.target && previous.key == self.key {
            let target = std::mem::take(&mut self.target);
            *self = Compare { target, ..previous.clone() };
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub repo: bool,
    pub root: String,
    /// Carpeta del proyecto dentro del repositorio ("" o "plugins/").
    pub prefix: String,
    /// merge, rebase, cherry-pick o revert en curso ("" si no).
    pub operation: String,
    /// Mensaje que propone git para el commit de la fusión.
    pub merge_msg: String,
    /// Rama; con HEAD separado, el commit abreviado.
    pub branch: String,
    pub detached: bool,
    /// Rama sin commits todavía.
    pub initial: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub merge: Vec<Change>,
    pub staged: Vec<Change>,
    pub changes: Vec<Change>,
    /// Ignorados por .gitignore; las carpetas terminan en "/".
    pub ignored: Vec<String>,
    pub compare: Compare,
}

impl Status {
    pub fn parse(output: &str) -> Status {
        let mut status = Status::default();
        let mut records = output.split('\0');
        for record in records.by_ref() {
            if record == "--" {
                break;
            }
            let Some((key, value)) = record.split_once('=') else { continue };
            let value = value.to_string();
            let c = &mut status.compare;
            match key {
                "repo" => status.repo = value == "1",
                "root" => status.root = value,
                "prefix" => status.prefix = value,
                "operation" => status.operation = value,
                "merge_msg" => status.merge_msg = value,
                "target" => c.target = value,
                "check" => c.key = value,
                "tree" => c.tree = value,
                "conflict" => c.files.push(value),
                "ahead" => c.ahead = value.parse().unwrap_or(0),
                "behind" => c.behind = value.parse().unwrap_or(0),
                "merge" => {
                    c.check = match value.as_str() {
                        "same" => Check::Same,
                        "clean" => Check::Clean,
                        "conflicts" => Check::Conflicts,
                        "unrelated" => Check::Unrelated,
                        "missing" => Check::Missing,
                        "unsupported" => Check::Unsupported,
                        "error" => Check::Error,
                        _ => Check::None,
                    }
                }
                _ => {}
            }
        }
        let mut oid = String::new();
        while let Some(record) = records.next() {
            if let Some(header) = record.strip_prefix("# ") {
                let (key, value) = header.split_once(' ').unwrap_or((header, ""));
                match key {
                    "branch.oid" if value == "(initial)" => status.initial = true,
                    "branch.oid" => oid = value.to_string(),
                    "branch.head" if value == "(detached)" => status.detached = true,
                    "branch.head" => status.branch = value.to_string(),
                    "branch.upstream" => status.upstream = Some(value.to_string()),
                    "branch.ab" => {
                        for part in value.split_whitespace() {
                            let n = part[1..].parse().unwrap_or(0);
                            if part.starts_with('+') {
                                status.ahead = n;
                            } else {
                                status.behind = n;
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }
            let (tag, rest) = record.split_at(record.len().min(2));
            match tag {
                "1 " => status.ordinary(rest, 7, None),
                // Renombrado o copiado: la ruta anterior va en el registro siguiente.
                "2 " => {
                    let orig = records.next().map(str::to_string);
                    status.ordinary(rest, 8, orig);
                }
                "u " => {
                    if let Some(path) = rest.splitn(10, ' ').nth(9) {
                        status.merge.push(Change { path: path.to_string(), orig: None, kind: Kind::Conflict });
                    }
                }
                "? " => status.changes.push(Change { path: rest.to_string(), orig: None, kind: Kind::Untracked }),
                "! " => status.ignored.push(rest.to_string()),
                _ => {}
            }
        }
        if status.detached {
            status.branch = oid.chars().take(8).collect();
        }
        for list in [&mut status.merge, &mut status.staged, &mut status.changes] {
            list.sort_by(|a, b| a.path.cmp(&b.path));
        }
        status
    }

    /// "1 XY sub mH mI mW hH hI ruta" (con `fields` campos antes de la ruta).
    fn ordinary(&mut self, rest: &str, fields: usize, orig: Option<String>) {
        let mut parts = rest.splitn(fields + 1, ' ');
        let xy: Vec<char> = parts.next().unwrap_or_default().chars().collect();
        let Some(path) = parts.nth(fields - 1) else { return };
        let (Some(&x), Some(&y)) = (xy.first(), xy.get(1)) else { return };
        if let Some(kind) = Kind::from_code(x) {
            self.staged.push(Change { path: path.to_string(), orig: orig.clone(), kind });
        }
        if let Some(kind) = Kind::from_code(y) {
            // En el árbol de trabajo, un renombrado solo puede ser "intent to add".
            self.changes.push(Change { path: path.to_string(), orig: None, kind });
        }
    }

    /// Los archivos de un grupo; los de `Incoming` son los que chocarían.
    pub fn group(&self, group: Group) -> Vec<Change> {
        match group {
            Group::Merge => self.merge.clone(),
            Group::Staged => self.staged.clone(),
            Group::Changes => self.changes.clone(),
            Group::Incoming => self
                .compare
                .files
                .iter()
                .map(|path| Change { path: path.clone(), orig: None, kind: Kind::Conflict })
                .collect(),
        }
    }

    /// El mensaje que propone git para el commit de una fusión en curso.
    pub fn message_suggestion(&self) -> &str {
        if self.operation.is_empty() {
            ""
        } else {
            &self.merge_msg
        }
    }

    pub fn has_changes(&self) -> bool {
        !(self.merge.is_empty() && self.staged.is_empty() && self.changes.is_empty())
    }

    /// Para la barra de estado: hay cambios sin commit (VS Code añade "*").
    pub fn dirty(&self) -> bool {
        self.has_changes()
    }
}

/// Colores de git del explorador, con rutas relativas al proyecto.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Decorations {
    files: HashMap<PathBuf, Kind>,
    dirs: HashMap<PathBuf, Kind>,
    ignored: BTreeSet<PathBuf>,
}

impl Decorations {
    pub fn new(status: &Status) -> Decorations {
        let mut deco = Decorations::default();
        let inside = |path: &str| path.strip_prefix(status.prefix.as_str()).map(PathBuf::from);
        // Como VS Code: lo del árbol de trabajo tapa lo preparado y un
        // conflicto, todo lo demás.
        for change in status.staged.iter().chain(&status.changes).chain(&status.merge) {
            let Some(path) = inside(&change.path) else { continue };
            for dir in path.ancestors().skip(1).filter(|d| !d.as_os_str().is_empty()) {
                let kind = deco.dirs.entry(dir.to_path_buf()).or_insert(change.kind);
                if change.kind.weight() > kind.weight() {
                    *kind = change.kind;
                }
            }
            deco.files.insert(path, change.kind);
        }
        for path in &status.ignored {
            if let Some(path) = inside(path.trim_end_matches('/')) {
                deco.ignored.insert(path);
            }
        }
        deco
    }

    /// El estado de un archivo (o, en una carpeta, el de lo que tiene dentro).
    pub fn get(&self, path: &Path, is_dir: bool) -> Option<Kind> {
        if is_dir {
            self.dirs.get(path).copied()
        } else {
            self.files.get(path).copied()
        }
    }

    /// Ignorado por .gitignore, o dentro de una carpeta ignorada.
    pub fn ignored(&self, path: &Path) -> bool {
        !self.ignored.is_empty() && path.ancestors().any(|p| self.ignored.contains(p))
    }
}

/// Una fila de la vista, debajo de la cabecera de la rama.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// El mensaje del commit.
    Message,
    /// La rama con la que se comparan los conflictos.
    Compare,
    Header { group: Group, count: usize, expanded: bool },
    File { group: Group, change: Change },
}

impl Row {
    fn key(&self) -> Key {
        match self {
            Row::Message => Key::Message,
            Row::Compare => Key::Compare,
            Row::Header { group, .. } => Key::Header(*group),
            Row::File { group, change } => Key::File(*group, change.path.clone()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum Key {
    #[default]
    Message,
    Compare,
    Header(Group),
    File(Group, String),
}

/// La lista de la vista: grupos plegables y la fila elegida, que se conserva
/// al refrescar. Si desaparece (un archivo que se acaba de preparar), pasa a
/// la que ocupa su lugar, como en VS Code.
#[derive(Default)]
pub struct ScmList {
    rows: Vec<Row>,
    collapsed: BTreeSet<Group>,
    selected: Key,
    index: usize,
    pub scroll: usize,
}

impl ScmList {
    pub fn update(&mut self, status: &Status) {
        let mut rows = vec![Row::Message];
        if status.repo && !status.initial {
            rows.push(Row::Compare);
        }
        for group in GROUPS {
            let changes = status.group(group);
            if changes.is_empty() {
                continue;
            }
            let expanded = !self.collapsed.contains(&group);
            rows.push(Row::Header { group, count: changes.len(), expanded });
            if expanded {
                rows.extend(changes.into_iter().map(|change| Row::File { group, change }));
            }
        }
        let old = std::mem::replace(&mut self.rows, rows);
        let position = |key: &Key| self.rows.iter().position(|r| r.key() == *key);
        let index = position(&self.selected)
            .or_else(|| old.iter().skip(self.index + 1).find_map(|r| position(&r.key())))
            .or_else(|| old.iter().take(self.index).rev().find_map(|r| position(&r.key())))
            .unwrap_or(0);
        self.select(index);
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn selected(&self) -> Option<&Row> {
        self.rows.get(self.index)
    }

    pub fn select(&mut self, index: usize) {
        if let Some(row) = self.rows.get(index) {
            self.index = index;
            self.selected = row.key();
        }
    }

    pub fn select_message(&mut self) {
        self.select(0);
    }

    pub fn move_by(&mut self, delta: isize) {
        let last = self.rows.len().saturating_sub(1) as isize;
        self.select((self.index as isize + delta).clamp(0, last) as usize);
    }

    /// Pliega o despliega un grupo (la fila elegida pasa a su cabecera).
    pub fn set_expanded(&mut self, status: &Status, group: Group, expanded: bool) {
        if expanded {
            self.collapsed.remove(&group);
        } else {
            self.collapsed.insert(group);
        }
        self.selected = Key::Header(group);
        self.update(status);
    }

    pub fn keep_visible(&mut self, height: usize) {
        if height == 0 {
            return;
        }
        if self.index < self.scroll {
            self.scroll = self.index;
        } else if self.index >= self.scroll + height {
            self.scroll = self.index + 1 - height;
        }
        self.scroll = self.scroll.min(self.rows.len().saturating_sub(height));
    }

    pub fn scroll_by(&mut self, delta: isize, height: usize) {
        let max = self.rows.len().saturating_sub(height) as isize;
        self.scroll = (self.scroll as isize + delta).clamp(0, max.max(0)) as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(header: &[&str], porcelain: &[&str]) -> String {
        let mut records: Vec<&str> = header.to_vec();
        records.push("--");
        records.extend(porcelain);
        records.join("\0") + "\0"
    }

    const SHA: &str = "100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222";

    fn sample() -> Status {
        let modified = format!("1 .M N... {SHA} con espacio.txt");
        let both = format!("1 MM N... {SHA} src/app.rs");
        let renamed = format!("2 R. N... {SHA} R100 src/nuevo.rs");
        let added = format!("1 A. N... {SHA} docs/a.md");
        let conflict = format!("u UU N... 100644 100644 100644 100644 {} {} {} src/lib.rs", "a".repeat(40), "b".repeat(40), "c".repeat(40));
        Status::parse(&output(
            &["repo=1", "root=/r", "prefix=", "operation=merge", "merge_msg=Merge branch 'x'", "target=main",
              "check=aa:bb", "ahead=2", "behind=1", "merge=conflicts", "tree=ffff", "conflict=src/app.rs"],
            &["# branch.oid 0123456789abcdef", "# branch.head rama", "# branch.upstream origin/rama",
              "# branch.ab +3 -0", &modified, &both, &renamed, "src/viejo.rs", &added, &conflict,
              "? nuevo dir/x.txt", "! target/", "! .env"],
        ))
    }

    #[test]
    fn interpreta_la_cabecera_y_la_comprobacion() {
        let s = sample();
        assert!(s.repo && !s.detached && !s.initial);
        assert_eq!((s.branch.as_str(), s.upstream.as_deref(), s.ahead, s.behind), ("rama", Some("origin/rama"), 3, 0));
        assert_eq!((s.operation.as_str(), s.merge_msg.as_str()), ("merge", "Merge branch 'x'"));
        let c = &s.compare;
        assert_eq!((c.target.as_str(), c.check, c.tree.as_str(), c.ahead, c.behind), ("main", Check::Conflicts, "ffff", 2, 1));
        assert_eq!(c.files, ["src/app.rs"]);
    }

    #[test]
    fn reparte_en_grupos_como_vs_code() {
        let s = sample();
        fn paths(list: &[Change]) -> Vec<(&str, char)> {
            list.iter().map(|c| (c.path.as_str(), c.kind.letter())).collect()
        }
        assert_eq!(paths(&s.merge), [("src/lib.rs", '!')]);
        assert_eq!(paths(&s.staged), [("docs/a.md", 'A'), ("src/app.rs", 'M'), ("src/nuevo.rs", 'R')]);
        assert_eq!(paths(&s.changes), [("con espacio.txt", 'M'), ("nuevo dir/x.txt", 'U'), ("src/app.rs", 'M')]);
        assert_eq!(s.staged[2].orig.as_deref(), Some("src/viejo.rs"));
        assert_eq!(s.ignored, ["target/", ".env"]);
        assert_eq!(s.group(Group::Incoming)[0].kind, Kind::Conflict);
        assert_eq!(s.changes[1].name_and_dir(), ("x.txt", "nuevo dir"));
        assert_eq!(s.changes[0].name_and_dir(), ("con espacio.txt", ""));
    }

    #[test]
    fn sin_repositorio_head_separado_y_rama_nueva() {
        let none = Status::parse("repo=0\0");
        assert!(!none.repo && !none.has_changes());
        let detached = Status::parse(&output(&["repo=1"], &["# branch.oid 0123456789abcdef", "# branch.head (detached)"]));
        assert!(detached.detached);
        assert_eq!(detached.branch, "01234567");
        let initial = Status::parse(&output(&["repo=1"], &["# branch.oid (initial)", "# branch.head main"]));
        assert!(initial.initial);
        assert_eq!(initial.branch, "main");
    }

    #[test]
    fn el_resultado_anterior_vale_si_no_cambian_los_commits() {
        let previous = sample().compare;
        let mut same = Compare { target: "main".into(), key: "aa:bb".into(), check: Check::Same, ..Default::default() };
        same.keep_from(&previous);
        assert_eq!(same, previous);
        let mut other = Compare { target: "dev".into(), key: "aa:bb".into(), check: Check::Same, ..Default::default() };
        other.keep_from(&previous);
        assert_eq!(other.check, Check::Same);
        assert!(other.files.is_empty());
    }

    #[test]
    fn colores_del_explorador_relativos_al_proyecto() {
        let mut s = sample();
        let d = Decorations::new(&s);
        assert_eq!(d.get(Path::new("src/app.rs"), false), Some(Kind::Modified));
        assert_eq!(d.get(Path::new("src/lib.rs"), false), Some(Kind::Conflict));
        // Una carpeta toma el color de lo más importante que tiene dentro.
        assert_eq!(d.get(Path::new("src"), true), Some(Kind::Conflict));
        assert_eq!(d.get(Path::new("docs"), true), Some(Kind::Added));
        assert_eq!(d.get(Path::new("nuevo dir"), true), Some(Kind::Untracked));
        assert!(d.ignored(Path::new("target/release/x")) && d.ignored(Path::new(".env")));
        assert!(!d.ignored(Path::new("src")));
        // Proyecto dentro del repositorio: solo lo suyo, sin el prefijo.
        s.prefix = "src/".into();
        let d = Decorations::new(&s);
        assert_eq!(d.get(Path::new("app.rs"), false), Some(Kind::Modified));
        assert_eq!(d.get(Path::new("con espacio.txt"), false), None);
    }

    #[test]
    fn la_lista_conserva_la_eleccion_y_pasa_a_la_siguiente() {
        let mut s = sample();
        let mut list = ScmList::default();
        list.update(&s);
        let shown: Vec<String> = list
            .rows()
            .iter()
            .map(|r| match r {
                Row::Message => "msg".into(),
                Row::Compare => "cmp".into(),
                Row::Header { group, count, .. } => format!("{group:?} {count}"),
                Row::File { change, .. } => change.path.clone(),
            })
            .collect();
        assert_eq!(
            shown,
            ["msg", "cmp", "Merge 1", "src/lib.rs", "Staged 3", "docs/a.md", "src/app.rs", "src/nuevo.rs",
             "Changes 3", "con espacio.txt", "nuevo dir/x.txt", "src/app.rs", "Incoming 1", "src/app.rs"]
        );
        list.select(9);
        // Se prepara "con espacio.txt": la elección pasa a la siguiente.
        let staged = s.changes.remove(0);
        s.staged.push(staged);
        list.update(&s);
        assert!(matches!(list.selected(), Some(Row::File { change, .. }) if change.path == "nuevo dir/x.txt"));
        list.set_expanded(&s, Group::Staged, false);
        assert!(matches!(list.selected(), Some(Row::Header { group: Group::Staged, expanded: false, .. })));
        assert_eq!(list.rows().len(), shown.len() - 4);
        list.move_by(-100);
        assert_eq!(list.selected(), Some(&Row::Message));
    }

    #[test]
    fn sin_commits_no_hay_comparacion() {
        let s = Status::parse(&output(&["repo=1"], &["# branch.oid (initial)", "# branch.head main", "? a"]));
        let mut list = ScmList::default();
        list.update(&s);
        assert_eq!(list.rows()[1], Row::Header { group: Group::Changes, count: 1, expanded: true });
    }
}
