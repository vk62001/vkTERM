//! Árbol del explorador, con carpetas que se despliegan como en VS Code.
//!
//! Lógica pura, sin Zellij: el plugin lee el disco y le pasa los listados.
//! Las rutas son relativas a la raíz del proyecto ("" es la raíz), así que no
//! se puede salir de ella.
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Lo que VS Code oculta por defecto (`files.exclude`).
pub const EXCLUDED: [&str; 6] = [".git", ".svn", ".hg", "CVS", ".DS_Store", "Thumbs.db"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

/// Una fila visible del árbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
}

/// Lo que el plugin tiene que hacer después de una acción del usuario.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// Leer esta carpeta del disco (se acaba de desplegar).
    Load(PathBuf),
    /// Abrir este archivo en el editor.
    Open(PathBuf),
}

#[derive(Default)]
pub struct Tree {
    listings: HashMap<PathBuf, Vec<DirEntry>>,
    expanded: BTreeSet<PathBuf>,
    selected: Option<PathBuf>,
    pub show_excluded: bool,
    pub scroll: usize,
}

impl Tree {
    /// Guarda el contenido de una carpeta: primero carpetas, después archivos,
    /// sin distinguir mayúsculas (como VS Code).
    pub fn set_listing(&mut self, dir: &Path, mut entries: Vec<DirEntry>) {
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.name.cmp(&b.name))
        });
        self.listings.insert(dir.to_path_buf(), entries);
    }

    pub fn is_loaded(&self, dir: &Path) -> bool {
        self.listings.contains_key(dir)
    }

    /// Carpetas leídas que se ven ahora mismo (para refrescarlas).
    pub fn loaded_dirs(&self) -> Vec<PathBuf> {
        self.listings.keys().cloned().collect()
    }

    pub fn forget(&mut self, dir: &Path) {
        self.listings.remove(dir);
        self.expanded.remove(dir);
    }

    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        self.push_rows(Path::new(""), 0, &mut rows);
        rows
    }

    fn push_rows(&self, dir: &Path, depth: usize, rows: &mut Vec<Row>) {
        let Some(entries) = self.listings.get(dir) else { return };
        for entry in entries {
            if !self.show_excluded && EXCLUDED.contains(&entry.name.as_str()) {
                continue;
            }
            let path = dir.join(&entry.name);
            let expanded = entry.is_dir && self.expanded.contains(&path);
            rows.push(Row {
                path: path.clone(),
                name: entry.name.clone(),
                is_dir: entry.is_dir,
                depth,
                expanded,
            });
            if expanded {
                self.push_rows(&path, depth + 1, rows);
            }
        }
    }

    /// Índice de la fila seleccionada. Si la selección ya no se ve (se plegó
    /// su carpeta o se borró), pasa al antepasado visible más cercano.
    pub fn selected_index(&self, rows: &[Row]) -> usize {
        let mut wanted = self.selected.clone();
        while let Some(path) = wanted {
            if let Some(i) = rows.iter().position(|r| r.path == path) {
                return i;
            }
            wanted = path.parent().filter(|p| !p.as_os_str().is_empty()).map(Path::to_path_buf);
        }
        0
    }

    pub fn selected(&self) -> Option<Row> {
        let rows = self.rows();
        rows.get(self.selected_index(&rows)).cloned()
    }

    fn select_index(&mut self, rows: &[Row], index: usize) {
        self.selected = rows.get(index).map(|r| r.path.clone());
    }

    pub fn move_by(&mut self, delta: isize) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let current = self.selected_index(&rows) as isize;
        let target = (current + delta).clamp(0, rows.len() as isize - 1);
        self.select_index(&rows, target as usize);
    }

    pub fn home(&mut self) {
        let rows = self.rows();
        self.select_index(&rows, 0);
    }

    pub fn end(&mut self) {
        let rows = self.rows();
        self.select_index(&rows, rows.len().saturating_sub(1));
    }

    /// →: despliega la carpeta; si ya lo estaba, baja a su primer hijo.
    pub fn right(&mut self) -> Action {
        let Some(row) = self.selected() else { return Action::None };
        if !row.is_dir {
            return Action::None;
        }
        if !row.expanded {
            return self.expand(&row.path);
        }
        let rows = self.rows();
        let i = self.selected_index(&rows);
        if rows.get(i + 1).is_some_and(|next| next.depth > row.depth) {
            self.select_index(&rows, i + 1);
        }
        Action::None
    }

    /// ←: pliega la carpeta; si no, sube a la carpeta padre.
    pub fn left(&mut self) {
        let Some(row) = self.selected() else { return };
        if row.is_dir && row.expanded {
            self.expanded.remove(&row.path);
        } else if let Some(parent) = row.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            self.selected = Some(parent.to_path_buf());
        }
    }

    /// Enter: abre el archivo o pliega / despliega la carpeta.
    pub fn activate(&mut self) -> Action {
        match self.selected() {
            Some(row) if row.is_dir => self.toggle(&row.path),
            Some(row) => Action::Open(row.path),
            None => Action::None,
        }
    }

    /// Clic: selecciona la fila y la activa.
    pub fn click(&mut self, index: usize) -> Action {
        let rows = self.rows();
        if index >= rows.len() {
            return Action::None;
        }
        self.select_index(&rows, index);
        self.activate()
    }

    fn toggle(&mut self, dir: &Path) -> Action {
        if self.expanded.remove(dir) {
            Action::None
        } else {
            self.expand(dir)
        }
    }

    fn expand(&mut self, dir: &Path) -> Action {
        self.expanded.insert(dir.to_path_buf());
        if self.is_loaded(dir) {
            Action::None
        } else {
            Action::Load(dir.to_path_buf())
        }
    }

    /// Escribir una letra salta a la siguiente fila que empieza por ella.
    pub fn jump_to(&mut self, c: char) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let start = self.selected_index(&rows);
        let c = c.to_lowercase().next().unwrap_or(c);
        for offset in 1..=rows.len() {
            let i = (start + offset) % rows.len();
            if rows[i].name.to_lowercase().starts_with(c) {
                self.select_index(&rows, i);
                return;
            }
        }
    }

    /// Ajusta el desplazamiento para que la selección quepa en `height` filas.
    pub fn keep_visible(&mut self, height: usize) {
        let rows = self.rows();
        let i = self.selected_index(&rows);
        if height == 0 {
            return;
        }
        if i < self.scroll {
            self.scroll = i;
        } else if i >= self.scroll + height {
            self.scroll = i + 1 - height;
        }
        self.scroll = self.scroll.min(rows.len().saturating_sub(height));
    }

    pub fn scroll_by(&mut self, delta: isize, height: usize) {
        let max = self.rows().len().saturating_sub(height) as isize;
        self.scroll = (self.scroll as isize + delta).clamp(0, max.max(0)) as usize;
    }

    /// Carpetas desplegadas, de fuera hacia dentro (para la memoria).
    pub fn expanded_dirs(&self) -> Vec<PathBuf> {
        self.expanded.iter().cloned().collect()
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected.clone()
    }

    /// Vuelve a desplegar las carpetas de la memoria y la selección. Devuelve
    /// las carpetas que hay que leer del disco, de fuera hacia dentro: cada
    /// una se ve cuando está leída la anterior. Las que ya no existen las
    /// quita el plugin con `forget` al no poder leerlas.
    pub fn restore(&mut self, expanded: &[PathBuf], selected: Option<PathBuf>) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = expanded.to_vec();
        dirs.sort_by_key(|d| d.components().count());
        dirs.dedup();
        for dir in &dirs {
            self.expanded.insert(dir.clone());
        }
        if selected.is_some() {
            self.selected = selected;
        }
        dirs.into_iter().filter(|d| !self.is_loaded(d)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(list: &[(&str, bool)]) -> Vec<DirEntry> {
        list.iter().map(|(n, d)| DirEntry { name: n.to_string(), is_dir: *d }).collect()
    }

    fn demo() -> Tree {
        let mut t = Tree::default();
        t.set_listing(
            Path::new(""),
            entries(&[("README.md", false), (".git", true), ("src", true), ("Build", true), (".vkterm", false)]),
        );
        t.set_listing(Path::new("src"), entries(&[("main.rs", false), ("app", true), ("Lib.rs", false)]));
        t
    }

    fn names(t: &Tree) -> Vec<String> {
        t.rows().iter().map(|r| format!("{}{}", "  ".repeat(r.depth), r.name)).collect()
    }

    #[test]
    fn carpetas_primero_sin_mayusculas_y_sin_excluidos() {
        assert_eq!(names(&demo()), ["Build", "src", ".vkterm", "README.md"]);
    }

    #[test]
    fn ver_excluidos() {
        let mut t = demo();
        t.show_excluded = true;
        assert_eq!(names(&t), [".git", "Build", "src", ".vkterm", "README.md"]);
    }

    #[test]
    fn derecha_despliega_y_baja_al_primer_hijo() {
        let mut t = demo();
        t.move_by(1); // src
        assert_eq!(t.right(), Action::None); // ya estaba leída
        assert_eq!(names(&t), ["Build", "src", "  app", "  Lib.rs", "  main.rs", ".vkterm", "README.md"]);
        t.right();
        assert_eq!(t.selected().unwrap().path, PathBuf::from("src/app"));
        assert_eq!(t.right(), Action::Load(PathBuf::from("src/app")));
    }

    #[test]
    fn izquierda_pliega_o_sube_al_padre() {
        let mut t = demo();
        t.move_by(1);
        t.right();
        t.move_by(2); // src/Lib.rs
        t.left();
        assert_eq!(t.selected().unwrap().path, PathBuf::from("src"));
        t.left();
        assert_eq!(names(&t), ["Build", "src", ".vkterm", "README.md"]);
        t.left(); // en la raíz no se sube más
        assert_eq!(t.selected().unwrap().path, PathBuf::from("src"));
    }

    #[test]
    fn enter_abre_archivos_y_alterna_carpetas() {
        let mut t = demo();
        t.end();
        assert_eq!(t.activate(), Action::Open(PathBuf::from("README.md")));
        t.home();
        assert_eq!(t.activate(), Action::Load(PathBuf::from("Build")));
        assert_eq!(t.activate(), Action::None); // plegada otra vez
    }

    #[test]
    fn la_seleccion_sobrevive_a_plegar_y_refrescar() {
        let mut t = demo();
        t.move_by(1);
        t.right();
        t.move_by(3); // src/main.rs
        t.set_listing(Path::new("src"), entries(&[("new.rs", false), ("main.rs", false), ("app", true)]));
        assert_eq!(t.selected().unwrap().path, PathBuf::from("src/main.rs"));
        t.forget(Path::new("src"));
        assert_eq!(t.selected().unwrap().path, PathBuf::from("src"));
    }

    #[test]
    fn salto_por_letra() {
        let mut t = demo();
        t.jump_to('r');
        assert_eq!(t.selected().unwrap().name, "README.md");
        t.jump_to('S');
        assert_eq!(t.selected().unwrap().name, "src");
    }

    #[test]
    fn clic_selecciona_y_activa() {
        let mut t = demo();
        assert_eq!(t.click(3), Action::Open(PathBuf::from("README.md")));
        assert_eq!(t.click(99), Action::None);
    }

    #[test]
    fn restaurar_carpetas_y_seleccion() {
        let mut t = demo();
        let to_load = t.restore(
            &[PathBuf::from("src/app"), PathBuf::from("src"), PathBuf::from("Build")],
            Some(PathBuf::from("src/main.rs")),
        );
        // src ya estaba leída; las demás se leen de fuera hacia dentro.
        assert_eq!(to_load, [PathBuf::from("Build"), PathBuf::from("src/app")]);
        assert_eq!(names(&t), ["Build", "src", "  app", "  Lib.rs", "  main.rs", ".vkterm", "README.md"]);
        assert_eq!(t.selected().unwrap().path, PathBuf::from("src/main.rs"));
        assert_eq!(t.expanded_dirs(), [PathBuf::from("Build"), PathBuf::from("src"), PathBuf::from("src/app")]);
    }

    #[test]
    fn restaurar_una_carpeta_borrada() {
        let mut t = demo();
        t.restore(&[PathBuf::from("vieja")], Some(PathBuf::from("vieja/x.rs")));
        t.forget(Path::new("vieja")); // el plugin no pudo leerla
        assert!(t.expanded_dirs().is_empty());
        assert_eq!(t.selected().unwrap().path, PathBuf::from("Build"));
    }

    #[test]
    fn desplazamiento() {
        let mut t = demo();
        t.move_by(1);
        t.right();
        t.end();
        t.keep_visible(3);
        assert_eq!(t.scroll, 4);
        t.home();
        t.keep_visible(3);
        assert_eq!(t.scroll, 0);
    }
}
