//! Layout del workspace según qué zonas están visibles.
//!
//! Zellij no devuelve un panel oculto a su sitio al mostrarlo: lo inserta como
//! uno nuevo. Después de cada cambio, el plugin aplica este layout con
//! `override_layout`, que coloca cada panel existente en el hueco con su mismo
//! comando o plugin. Debe coincidir con config/zellij/layouts/vkterm.kdl.

/// Anchos y altos en porcentaje, como en el layout de arranque.
pub const SIDEBAR: u32 = 20;
pub const AGENTS: u32 = 25;
pub const PANEL: u32 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visible {
    pub sidebar: bool,
    pub panel: bool,
    pub agents: bool,
}

impl Default for Visible {
    fn default() -> Self {
        Visible { sidebar: true, panel: true, agents: true }
    }
}

pub fn kdl(v: Visible) -> String {
    let center = 100 - if v.sidebar { SIDEBAR } else { 0 } - if v.agents { AGENTS } else { 0 };
    let mut out = String::from("layout {\n    pane split_direction=\"vertical\" {\n");
    if v.sidebar {
        out += &format!(
            "        pane size=\"{SIDEBAR}%\" name=\"Explorador\" {{ plugin location=\"vkterm\"; }}\n"
        );
    }
    out += &format!("        pane size=\"{center}%\" split_direction=\"horizontal\" {{\n");
    if v.panel {
        out += &format!("            pane size=\"{}%\" name=\"Editor\" command=\"vkterm-editor\"\n", 100 - PANEL);
        out += &format!("            pane size=\"{PANEL}%\" name=\"Shell\" command=\"vkterm-shell\"\n");
    } else {
        out += "            pane name=\"Editor\" command=\"vkterm-editor\"\n";
    }
    out += "        }\n";
    if v.agents {
        out += &format!("        pane size=\"{AGENTS}%\" split_direction=\"horizontal\" {{\n");
        out += "            pane name=\"Claude Code\" command=\"claude\" start_suspended=true\n";
        out += "            pane name=\"Codex\" command=\"codex\" start_suspended=true\n";
        out += "        }\n";
    }
    out += "    }\n    pane size=1 borderless=true { plugin location=\"compact-bar\"; }\n}\n";
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_visible() {
        let k = kdl(Visible::default());
        assert!(k.contains("size=\"20%\" name=\"Explorador\""));
        assert!(k.contains("pane size=\"55%\" split_direction=\"horizontal\""));
        assert!(k.contains("size=\"70%\" name=\"Editor\""));
        assert!(k.contains("size=\"30%\" name=\"Shell\""));
        assert!(k.contains("size=\"25%\" split_direction"));
    }

    #[test]
    fn sin_barra_lateral_el_centro_ocupa_su_espacio() {
        let k = kdl(Visible { sidebar: false, ..Default::default() });
        assert!(!k.contains("Explorador"));
        assert!(k.contains("pane size=\"75%\" split_direction=\"horizontal\""));
    }

    #[test]
    fn sin_panel_el_editor_ocupa_toda_la_columna() {
        let k = kdl(Visible { panel: false, ..Default::default() });
        assert!(!k.contains("Shell"));
        assert!(k.contains("pane name=\"Editor\" command=\"vkterm-editor\""));
    }

    #[test]
    fn solo_editor() {
        let k = kdl(Visible { sidebar: false, panel: false, agents: false });
        assert!(k.contains("pane size=\"100%\" split_direction=\"horizontal\""));
        assert!(!k.contains("Claude") && !k.contains("Codex") && !k.contains("Explorador"));
    }

    #[test]
    fn coincide_con_el_layout_de_arranque() {
        let base = include_str!("../../../config/zellij/layouts/vkterm.kdl");
        for line in kdl(Visible::default()).lines().filter(|l| l.contains("name=")) {
            assert!(base.contains(line.trim()), "falta en vkterm.kdl: {}", line.trim());
        }
    }
}
