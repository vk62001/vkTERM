//! Resultados de la vista «Buscar» (⌘⇧F), como la de VS Code: lo que imprime
//! `bin/vkterm-search` (una línea «ruta:línea:columna:texto» por
//! coincidencia) agrupado por archivo, y las filas de la vista: el campo de
//! búsqueda, el de reemplazo y los resultados plegables.
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    /// Columna (1 = primera), en bytes, como la da ripgrep.
    pub col: usize,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hits {
    pub path: String,
    pub matches: Vec<Match>,
}

impl Hits {
    /// («nombre», «carpeta») de la ruta.
    pub fn name_and_dir(&self) -> (&str, &str) {
        match self.path.rsplit_once('/') {
            Some((dir, name)) => (name, dir),
            None => (&self.path, ""),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Query,
    Replace,
    File(usize),
    Match(usize, usize),
}

#[derive(Default)]
pub struct Results {
    pub files: Vec<Hits>,
    pub total: usize,
    /// `bin/vkterm-search` corta en 2000 coincidencias.
    pub truncated: bool,
}

pub const LIMIT: usize = 2000;

impl Results {
    pub fn parse(output: &str) -> Results {
        let mut results = Results::default();
        for line in output.lines() {
            let mut parts = line.splitn(4, ':');
            let (Some(path), Some(number), Some(col), Some(text)) = (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let (Ok(number), Ok(col)) = (number.parse(), col.parse()) else { continue };
            if results.files.last().map(|f| f.path.as_str()) != Some(path) {
                results.files.push(Hits { path: path.to_string(), matches: Vec::new() });
            }
            if let Some(file) = results.files.last_mut() {
                file.matches.push(Match { line: number, col, text: text.to_string() });
            }
            results.total += 1;
        }
        results.truncated = results.total >= LIMIT;
        results
    }

    /// Las filas de la vista: los campos y, bajo ellos, cada archivo con sus
    /// coincidencias (salvo los plegados).
    pub fn rows(&self, show_replace: bool, collapsed: &BTreeSet<String>) -> Vec<Row> {
        let mut rows = vec![Row::Query];
        if show_replace {
            rows.push(Row::Replace);
        }
        for (f, file) in self.files.iter().enumerate() {
            rows.push(Row::File(f));
            if !collapsed.contains(&file.path) {
                rows.extend((0..file.matches.len()).map(|m| Row::Match(f, m)));
            }
        }
        rows
    }
}

/// El texto de una coincidencia sin la sangría, y cuántos caracteres se
/// quitaron (para colocar el resaltado).
pub fn trimmed(text: &str) -> (&str, usize) {
    let trimmed = text.trim_start();
    (trimmed, text.chars().count() - trimmed.chars().count())
}

/// Posición en caracteres de una columna en bytes (1 = la primera).
pub fn char_index(text: &str, col: usize) -> usize {
    let byte = col.saturating_sub(1).min(text.len());
    let byte = (0..=byte).rev().find(|&b| text.is_char_boundary(b)).unwrap_or(0);
    text[..byte].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUT: &str = "src/a.rs:3:5:let hola = 1;\nsrc/a.rs:9:1:// hola: mundo\nREADME.md:1:3:x hola\nbasura\n";

    #[test]
    fn agrupa_por_archivo_y_conserva_los_dos_puntos_del_texto() {
        let r = Results::parse(OUT);
        assert_eq!(r.total, 3);
        assert_eq!(r.files.len(), 2);
        assert_eq!(r.files[0].path, "src/a.rs");
        assert_eq!(r.files[0].matches[1], Match { line: 9, col: 1, text: "// hola: mundo".into() });
        assert_eq!(r.files[1].name_and_dir(), ("README.md", ""));
        assert_eq!(r.files[0].name_and_dir(), ("a.rs", "src"));
        assert!(!r.truncated);
    }

    #[test]
    fn las_filas_siguen_el_pliegue() {
        let r = Results::parse(OUT);
        let none = BTreeSet::new();
        assert_eq!(
            r.rows(false, &none),
            vec![Row::Query, Row::File(0), Row::Match(0, 0), Row::Match(0, 1), Row::File(1), Row::Match(1, 0)]
        );
        let collapsed = BTreeSet::from(["src/a.rs".to_string()]);
        assert_eq!(r.rows(true, &collapsed), vec![Row::Query, Row::Replace, Row::File(0), Row::File(1), Row::Match(1, 0)]);
    }

    #[test]
    fn sin_resultados_solo_los_campos() {
        assert_eq!(Results::parse("").rows(false, &BTreeSet::new()), vec![Row::Query]);
    }

    #[test]
    fn columnas_en_bytes_a_caracteres() {
        assert_eq!(char_index("añb", 1), 0);
        assert_eq!(char_index("añb", 4), 2);
        assert_eq!(char_index("añb", 99), 3);
        assert_eq!(trimmed("   x y"), ("x y", 3));
    }
}
