//! Temas de color (⌘K ⌘T), como "Preferences: Color Theme" de VS Code. Cada
//! tema reúne lo que necesita cada pieza: el tema de Ghostty, el de Zellij, el
//! esquema de Micro y los colores de los paneles sin foco.
//!
//! bin/vkterm-theme guarda el elegido en config/local (o VKTERM_STATE_DIR) y lo
//! aplica a Ghostty y a Micro; el plugin cambia el de Zellij (`reconfigure`).

pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub dark: bool,
    /// Tema de Ghostty: uno de los suyos o un archivo de config/ghostty/themes.
    pub ghostty: &'static str,
    /// Tema de Zellij: uno de los suyos o de config/zellij/themes.
    pub zellij: &'static str,
    /// Color del texto y del fondo de los paneles sin foco.
    pub inactive_fg: &'static str,
    pub inactive_bg: &'static str,
}

impl Theme {
    /// Esquema de Micro (config/micro/colorschemes): usa la paleta del
    /// terminal, solo cambian los contrastes de los temas claros.
    pub fn micro(&self) -> &'static str {
        if self.dark {
            "vkterm"
        } else {
            "vkterm-light"
        }
    }
}

pub const DEFAULT: &str = "catppuccin-mocha";

pub const THEMES: &[Theme] = &[
    Theme {
        id: "catppuccin-mocha",
        name: "Catppuccin Mocha",
        dark: true,
        ghostty: "Catppuccin Mocha",
        zellij: "catppuccin-mocha",
        inactive_fg: "#6c7086",
        inactive_bg: "#11111b",
    },
    Theme {
        id: "catppuccin-latte",
        name: "Catppuccin Latte",
        dark: false,
        ghostty: "Catppuccin Latte",
        zellij: "catppuccin-latte",
        inactive_fg: "#9ca0b0",
        inactive_bg: "#dce0e8",
    },
    Theme {
        id: "dark-modern",
        name: "Dark Modern (VS Code)",
        dark: true,
        ghostty: "Dark Modern",
        zellij: "vkterm-dark-modern",
        inactive_fg: "#6e6e6e",
        inactive_bg: "#141414",
    },
    Theme {
        id: "light-modern",
        name: "Light Modern (VS Code)",
        dark: false,
        ghostty: "Light Modern",
        zellij: "vkterm-light-modern",
        inactive_fg: "#8b8b8b",
        inactive_bg: "#ececec",
    },
    Theme {
        id: "dracula",
        name: "Dracula",
        dark: true,
        ghostty: "Dracula",
        zellij: "dracula",
        inactive_fg: "#6272a4",
        inactive_bg: "#1e1f29",
    },
    Theme {
        id: "nord",
        name: "Nord",
        dark: true,
        ghostty: "Nord",
        zellij: "nord",
        inactive_fg: "#616e88",
        inactive_bg: "#242933",
    },
    Theme {
        id: "tokyo-night",
        name: "Tokyo Night",
        dark: true,
        ghostty: "TokyoNight",
        zellij: "tokyo-night",
        inactive_fg: "#565f89",
        inactive_bg: "#13141c",
    },
    Theme {
        id: "gruvbox-dark",
        name: "Gruvbox Dark",
        dark: true,
        ghostty: "Gruvbox Dark",
        zellij: "gruvbox-dark",
        inactive_fg: "#7c6f64",
        inactive_bg: "#1d2021",
    },
    Theme {
        id: "gruvbox-light",
        name: "Gruvbox Light",
        dark: false,
        ghostty: "Gruvbox Light",
        zellij: "gruvbox-light",
        inactive_fg: "#a89984",
        inactive_bg: "#ebdbb2",
    },
];

/// El tema con ese id; si no existe (o está vacío), el predeterminado.
pub fn find(id: &str) -> &'static Theme {
    let id = id.trim();
    THEMES
        .iter()
        .find(|t| t.id == id)
        .or_else(|| THEMES.iter().find(|t| t.id == DEFAULT))
        .expect("el tema predeterminado existe")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn is_hex(color: &str) -> bool {
        color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn ids_unicos_y_colores_validos() {
        let ids: HashSet<_> = THEMES.iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), THEMES.len());
        for theme in THEMES {
            assert!(is_hex(theme.inactive_fg) && is_hex(theme.inactive_bg), "{}", theme.id);
            assert!(!theme.ghostty.is_empty() && !theme.zellij.is_empty());
        }
    }

    #[test]
    fn el_predeterminado_existe_y_es_el_de_siempre() {
        assert_eq!(find("").id, DEFAULT);
        assert_eq!(find("no-existe").id, DEFAULT);
        assert_eq!(find(" dracula\n").id, "dracula");
        assert_eq!(find(DEFAULT).ghostty, "Catppuccin Mocha");
    }

    #[test]
    fn micro_usa_el_esquema_claro_en_los_temas_claros() {
        assert_eq!(find("catppuccin-latte").micro(), "vkterm-light");
        assert_eq!(find("dark-modern").micro(), "vkterm");
    }
}
