//! «Nuevo archivo…» (⌘N), como el del explorador de VS Code: el nombre se
//! escribe en una ventana (src/prompt.rs) y el archivo se crea en la carpeta
//! seleccionada en el explorador. Admite subcarpetas: con `ui/Boton.tsx`
//! también se crea `ui` si no existe. Aquí, la carpeta de destino y lo que
//! impide crear el archivo.
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`. Las rutas son
//! relativas a la raíz del proyecto ("" es la raíz).
use std::path::{Component, Path, PathBuf};

/// Carpeta donde se crea: la seleccionada o, si es un archivo, la suya. Sin
/// selección, la raíz.
pub fn folder(selected: Option<(&Path, bool)>) -> PathBuf {
    match selected {
        Some((path, true)) => path.to_path_buf(),
        Some((path, false)) => path.parent().map(Path::to_path_buf).unwrap_or_default(),
        None => PathBuf::new(),
    }
}

/// Lo que impide crear el archivo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Aún no se ha escrito nada.
    Empty,
    /// Termina en "/": falta el nombre del archivo.
    NoFileName,
    /// Ruta absoluta, ".", ".." o una barra de más.
    Invalid,
    /// Ya hay un archivo o carpeta con esa ruta.
    Exists(PathBuf),
    /// Una de las carpetas del camino es un archivo.
    NotAFolder(PathBuf),
}

impl Problem {
    pub fn message(&self, name: &str) -> String {
        match self {
            Problem::Empty => String::new(),
            Problem::NoFileName => "Falta el nombre del archivo después de la última /".to_string(),
            Problem::Invalid => format!("«{}» no es un nombre de archivo válido", name.trim()),
            Problem::Exists(path) => format!("Ya existe {}: elige otro nombre", path.display()),
            Problem::NotAFolder(path) => format!("{} es un archivo, no una carpeta", path.display()),
        }
    }
}

/// El archivo que se crearía al escribir `name` en `folder`. `kind` dice si
/// una ruta del proyecto existe: Some(true) si es una carpeta, Some(false) si
/// es un archivo.
pub fn target(folder: &Path, name: &str, kind: impl Fn(&Path) -> Option<bool>) -> Result<PathBuf, Problem> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Problem::Empty);
    }
    if name.ends_with('/') {
        return Err(Problem::NoFileName);
    }
    let relative = Path::new(name);
    let normal = relative.components().all(|c| matches!(c, Component::Normal(_)));
    if !normal || name.contains("//") || name.contains('\0') {
        return Err(Problem::Invalid);
    }
    let path = folder.join(relative);
    // De fuera hacia dentro: la primera carpeta del camino que es un archivo.
    let mut ancestors: Vec<&Path> = path.ancestors().skip(1).filter(|a| a.starts_with(folder) && a != &folder).collect();
    ancestors.reverse();
    if let Some(file) = ancestors.into_iter().find(|a| kind(a) == Some(false)) {
        return Err(Problem::NotAFolder(file.to_path_buf()));
    }
    if kind(&path).is_some() {
        return Err(Problem::Exists(path));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un proyecto con src/ (carpeta), src/main.rs y README.md.
    fn kind(path: &Path) -> Option<bool> {
        match path.to_str()? {
            "src" => Some(true),
            "src/main.rs" | "README.md" => Some(false),
            _ => None,
        }
    }

    #[test]
    fn la_carpeta_de_la_seleccion() {
        assert_eq!(folder(Some((Path::new("src"), true))), Path::new("src"));
        assert_eq!(folder(Some((Path::new("src/main.rs"), false))), Path::new("src"));
        assert_eq!(folder(Some((Path::new("README.md"), false))), Path::new(""));
        assert_eq!(folder(None), Path::new(""));
    }

    #[test]
    fn crea_en_la_carpeta_y_con_subcarpetas() {
        assert_eq!(target(Path::new("src"), "lib.rs", kind), Ok(PathBuf::from("src/lib.rs")));
        assert_eq!(target(Path::new(""), " notas.md ", kind), Ok(PathBuf::from("notas.md")));
        assert_eq!(target(Path::new(""), "src/ui/boton.rs", kind), Ok(PathBuf::from("src/ui/boton.rs")));
    }

    #[test]
    fn lo_que_impide_crearlo() {
        assert_eq!(target(Path::new(""), "  ", kind), Err(Problem::Empty));
        assert_eq!(target(Path::new(""), "src/", kind), Err(Problem::NoFileName));
        for name in ["/etc/hosts", "../fuera.rs", "a/../b.rs", "./a.rs", "a//b.rs"] {
            assert_eq!(target(Path::new("src"), name, kind), Err(Problem::Invalid), "{name}");
        }
        assert_eq!(target(Path::new("src"), "main.rs", kind), Err(Problem::Exists(PathBuf::from("src/main.rs"))));
        assert_eq!(target(Path::new(""), "src", kind), Err(Problem::Exists(PathBuf::from("src"))));
        assert_eq!(
            target(Path::new(""), "README.md/a.rs", kind),
            Err(Problem::NotAFolder(PathBuf::from("README.md")))
        );
        assert!(Problem::Exists(PathBuf::from("src/main.rs")).message("main.rs").contains("src/main.rs"));
    }
}
