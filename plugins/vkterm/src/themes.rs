//! Temas de color (⌘K ⌘T), como "Preferences: Color Theme" de VS Code. Cada
//! tema es uno de VS Code y de él salen, con el mismo nombre (`vkterm-<id>`):
//! - la paleta del terminal de Ghostty (config/ghostty/themes);
//! - el resaltado y la interfaz del editor (config/micro/colorschemes);
//! - la interfaz de Zellij y del plugin: barra lateral, listas, barra de
//!   estado y marcos (config/zellij/themes/vkterm.kdl).
//!
//! scripts/gen-themes.py los genera, con esta tabla (src/themes_data.rs), a
//! partir de config/themes/themes.json. bin/vkterm-theme guarda el elegido en
//! config/local (o VKTERM_STATE_DIR) y lo aplica a Ghostty y a Micro; el
//! explorador cambia el de Zellij (`reconfigure`).
pub use crate::themes_data::{DEFAULT, THEMES};
use crate::style::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dark,
    Light,
    HighContrastDark,
    HighContrastLight,
}

impl Kind {
    pub fn is_dark(self) -> bool {
        matches!(self, Kind::Dark | Kind::HighContrastDark)
    }

    /// Grupo del selector, en el orden de VS Code: claros, oscuros y alto
    /// contraste.
    pub fn group(self) -> &'static str {
        match self {
            Kind::Light => "temas claros",
            Kind::Dark => "temas oscuros",
            Kind::HighContrastDark | Kind::HighContrastLight => "temas de alto contraste",
        }
    }
}

/// Orden de los grupos en el selector.
pub const GROUPS: [&str; 3] = ["temas claros", "temas oscuros", "temas de alto contraste"];

pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    /// Atenuar los paneles sin foco (no en los de alto contraste).
    pub dim: bool,
    /// Color del texto y del fondo de los paneles sin foco.
    pub inactive_fg: &'static str,
    pub inactive_bg: &'static str,
    /// Colores de git (gitDecoration.*ResourceForeground), en el orden de
    /// GIT_COLORS en scripts/gen-themes.py: ver `git_colors`.
    pub git: [&'static str; 7],
}

/// Los colores de git del tema: los de los archivos en la vista Control de
/// código fuente y en el explorador.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GitColors {
    pub modified: Rgb,
    pub added: Rgb,
    pub deleted: Rgb,
    pub renamed: Rgb,
    pub untracked: Rgb,
    pub ignored: Rgb,
    pub conflicting: Rgb,
}

impl Theme {
    /// Nombre del tema en Ghostty, Zellij y Micro.
    pub fn file(&self) -> String {
        format!("vkterm-{}", self.id)
    }

    pub fn git_colors(&self) -> GitColors {
        let [modified, added, deleted, renamed, untracked, ignored, conflicting] =
            self.git.map(|hex| Rgb::parse(hex).unwrap_or_default());
        GitColors { modified, added, deleted, renamed, untracked, ignored, conflicting }
    }
}

/// El tema con ese id; si no existe (o está vacío), el predeterminado.
pub fn find(id: &str) -> &'static Theme {
    &THEMES[index(id)]
}

/// Posición en THEMES del tema con ese id (o del predeterminado).
pub fn index(id: &str) -> usize {
    let id = id.trim();
    THEMES
        .iter()
        .position(|t| t.id == id)
        .or_else(|| THEMES.iter().position(|t| t.id == DEFAULT))
        .expect("el tema predeterminado existe")
}

/// Los temas en el orden del selector: por grupo y, dentro de cada uno, por
/// nombre, como VS Code.
pub fn picker_order() -> Vec<usize> {
    let mut order: Vec<usize> = (0..THEMES.len()).collect();
    order.sort_by_key(|&i| {
        let theme = &THEMES[i];
        let group = GROUPS.iter().position(|g| *g == theme.kind.group()).unwrap_or(GROUPS.len());
        (group, theme.name.to_lowercase())
    });
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    fn is_hex(color: &str) -> bool {
        color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn ids_unicos_y_colores_validos() {
        let ids: HashSet<_> = THEMES.iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), THEMES.len());
        for theme in THEMES {
            assert!(is_hex(theme.inactive_fg) && is_hex(theme.inactive_bg), "{}", theme.id);
            assert!(theme.git.iter().all(|c| is_hex(c)), "{}: colores de git", theme.id);
        }
    }

    #[test]
    fn el_predeterminado_existe_y_es_el_de_siempre() {
        assert_eq!(find("").id, DEFAULT);
        assert_eq!(find("no-existe").id, DEFAULT);
        assert_eq!(find(" dracula\n").id, "dracula");
        assert_eq!(DEFAULT, "catppuccin-mocha");
        assert_eq!(THEMES[0].id, DEFAULT);
    }

    #[test]
    fn cada_tema_tiene_sus_archivos_generados() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let zellij = std::fs::read_to_string(root.join("config/zellij/themes/vkterm.kdl")).unwrap();
        for theme in THEMES {
            let name = theme.file();
            assert!(root.join("config/ghostty/themes").join(&name).is_file(), "{name}");
            assert!(root.join(format!("config/micro/colorschemes/{name}.micro")).is_file(), "{name}");
            assert!(zellij.contains(&format!("    {name} {{")), "{name}");
        }
    }

    #[test]
    fn el_selector_agrupa_como_vs_code() {
        let order = picker_order();
        assert_eq!(order.len(), THEMES.len());
        let groups: Vec<&str> = order.iter().map(|&i| THEMES[i].kind.group()).collect();
        let mut sorted = groups.clone();
        sorted.sort_by_key(|g| GROUPS.iter().position(|x| x == g));
        assert_eq!(groups, sorted);
        assert_eq!(THEMES[order[0]].kind, Kind::Light);
        assert!(THEMES[*order.last().unwrap()].kind == Kind::HighContrastDark
            || THEMES[*order.last().unwrap()].kind == Kind::HighContrastLight);
    }

    #[test]
    fn los_de_alto_contraste_no_se_atenuan() {
        for theme in THEMES {
            let high_contrast = matches!(theme.kind, Kind::HighContrastDark | Kind::HighContrastLight);
            assert_eq!(theme.dim, !high_contrast, "{}", theme.id);
        }
    }
}
