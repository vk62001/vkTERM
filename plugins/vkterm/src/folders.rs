//! Las carpetas de «Abrir reciente» (⌘O), como en VS Code: las de
//! `vkterm-state recent`, la última usada primero, con su nombre y la
//! carpeta de arriba (con ~ por la carpeta personal).
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    None,
    /// Abierta en otra pestaña: con Enter se trae su IDE; con ⇧Enter se va a ella.
    Open,
    /// Ya no existe (o su disco no está conectado).
    Missing,
}

impl Mark {
    /// Lo que se muestra a la derecha.
    pub fn text(self) -> &'static str {
        match self {
            Mark::None => "",
            Mark::Open => "abierta",
            Mark::Missing => "no disponible",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub path: String,
    pub name: String,
    pub parent: String,
    pub mark: Mark,
}

impl Folder {
    /// Lo que se filtra y se muestra: "nombre  carpeta de arriba".
    pub fn label(&self) -> String {
        if self.parent.is_empty() {
            self.name.clone()
        } else {
            format!("{}  {}", self.name, self.parent)
        }
    }
}

/// Las líneas "RUTA<tab>ESTADO" de `vkterm-state recent`.
pub fn parse(text: &str, home: &str) -> Vec<Folder> {
    text.lines()
        .filter_map(|line| {
            let (path, mark) = line.split_once('\t').unwrap_or((line, ""));
            if path.is_empty() {
                return None;
            }
            let mark = match mark.trim() {
                "open" => Mark::Open,
                "missing" => Mark::Missing,
                _ => Mark::None,
            };
            let (name, parent) = split(path, home);
            Some(Folder { path: path.to_string(), name, parent, mark })
        })
        .collect()
}

fn split(path: &str, home: &str) -> (String, String) {
    let under_home = !home.is_empty() && (path == home || path.starts_with(&format!("{home}/")));
    let shown = if under_home { format!("~{}", &path[home.len()..]) } else { path.to_string() };
    if shown == "/" || shown == "~" {
        return (shown, String::new());
    }
    match shown.rsplit_once('/') {
        Some(("", name)) => (name.to_string(), "/".to_string()),
        Some((parent, name)) => (name.to_string(), parent.to_string()),
        None => (shown, String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombre_carpeta_de_arriba_y_estado() {
        let list = parse("/Users/ana/src/web\topen\n/Volumes/Apps/api\t\n/Volumes/USB/viejo\tmissing\n", "/Users/ana");
        let shown: Vec<_> = list.iter().map(|f| (f.label(), f.mark)).collect();
        assert_eq!(
            shown,
            [
                ("web  ~/src".to_string(), Mark::Open),
                ("api  /Volumes/Apps".to_string(), Mark::None),
                ("viejo  /Volumes/USB".to_string(), Mark::Missing),
            ]
        );
        assert_eq!(list[0].path, "/Users/ana/src/web");
    }

    #[test]
    fn la_carpeta_personal_la_raiz_y_lineas_vacias() {
        let list = parse("/Users/ana\t\n\n/\t\n/tmp\t\n/Users/anabel/x\t\n", "/Users/ana");
        let labels: Vec<_> = list.iter().map(Folder::label).collect();
        assert_eq!(labels, ["~", "/", "tmp  /", "x  /Users/anabel"]);
    }
}
