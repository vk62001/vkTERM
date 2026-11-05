//! Búsqueda en todo el proyecto (⌘⇧F): la línea de `rg --vimgrep` (o de
//! `git grep -n --column`) y el script de «Reemplazar en todos los archivos».
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

/// `ruta:línea:columna:texto` (ripgrep con --column) o `ruta:línea:texto`
/// (`git grep -n`). Las rutas con `:` no se esperan en un proyecto.
pub fn parse_line(raw: &str) -> Option<Hit> {
    let mut parts = raw.splitn(4, ':');
    let path = parts.next().filter(|p| !p.is_empty())?;
    let line: usize = parts.next()?.parse().ok()?;
    let third = parts.next()?;
    let fourth = parts.next();
    // Con columna (ripgrep) el texto es el cuarto campo; sin ella (git grep),
    // el tercero y lo que le sigue.
    let text = match (third.parse::<usize>(), fourth) {
        (Ok(_column), Some(text)) => text.to_string(),
        (_, Some(rest)) => format!("{third}:{rest}"),
        (_, None) => third.to_string(),
    };
    Some(Hit { path: path.to_string(), line, text: text.trim().to_string() })
}

/// Resultados agrupados por archivo en el orden en que llegan.
pub fn files(hits: &[Hit]) -> Vec<&str> {
    let mut seen: Vec<&str> = Vec::new();
    for hit in hits {
        if !seen.contains(&hit.path.as_str()) {
            seen.push(&hit.path);
        }
    }
    seen
}

/// Texto de una fila: `ruta:línea  texto`, en `width` columnas.
pub fn row_text(hit: &Hit, width: usize) -> String {
    let head = format!("{}:{}", hit.path, hit.line);
    format!("{head}  {}", hit.text).chars().take(width).collect()
}

/// Mayúsculas y minúsculas según smart-case, como ripgrep y VS Code con
/// «Usar expresión»: todo en minúscula ignora la caja.
pub fn ignore_case(query: &str) -> bool {
    query == query.to_lowercase()
}

/// Script de perl que reemplaza `$ENV{Q}` por `$ENV{R}` (literales) en los
/// archivos que recibe como argumentos.
pub const REPLACE_PERL: &str = "BEGIN{$q=$ENV{Q};$r=$ENV{R};$re=($q eq lc $q)?qr/\\Q$q\\E/i:qr/\\Q$q\\E/} s/$re/$r/g";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linea_de_ripgrep_con_columna() {
        let hit = parse_line("src/main.rs:12:5:    let x = 1; // a:b").unwrap();
        assert_eq!(hit, Hit { path: "src/main.rs".into(), line: 12, text: "let x = 1; // a:b".into() });
    }

    #[test]
    fn linea_de_git_grep_sin_columna() {
        let hit = parse_line("a.txt:3:hola: mundo").unwrap();
        assert_eq!((hit.path.as_str(), hit.line, hit.text.as_str()), ("a.txt", 3, "hola: mundo"));
        let hit = parse_line("a.txt:3:7 tiene 7").unwrap();
        assert_eq!(hit.text, "7 tiene 7");
    }

    #[test]
    fn lineas_que_no_son_resultados() {
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line("sin dos puntos"), None);
        assert_eq!(parse_line("a:b:c:d"), None);
    }

    #[test]
    fn archivos_sin_repetir() {
        let hits = ["a:1:1:x", "a:2:1:y", "b:1:1:z"].iter().filter_map(|l| parse_line(l)).collect::<Vec<_>>();
        assert_eq!(files(&hits), vec!["a", "b"]);
    }

    #[test]
    fn smart_case() {
        assert!(ignore_case("foo"));
        assert!(!ignore_case("Foo"));
    }
}
