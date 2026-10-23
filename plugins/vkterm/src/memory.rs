//! Memoria del workspace de cada carpeta, como VS Code: qué zonas se ven, qué
//! panel tiene el foco y qué carpetas del explorador están desplegadas. El
//! plugin la guarda con `vkterm-state save workspace …` cuando cambia y la
//! aplica una vez al abrir la carpeta (bin/vkterm-state load).
//!
//! Texto de líneas clave=valor, como el resto del estado de vkTERM. Lógica
//! pura, sin Zellij: se prueba con `cargo test --lib`.
use std::path::{Component, Path, PathBuf};

use crate::layout::Visible;

/// Paneles a los que se puede devolver el foco (como `focus` en config.kdl).
pub const FOCUS_TARGETS: [&str; 5] = ["explorer", "editor", "claude", "codex", "shell"];

/// Acciones de `vkterm --then` (los atajos del modo terminal).
pub const THEN_ACTIONS: [&str; 4] = ["show_sidebar", "show_panel", "quick_open", "command_palette"];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Memory {
    pub visible: Visible,
    pub focus: Option<String>,
    /// Carpetas desplegadas, relativas al proyecto.
    pub expanded: Vec<PathBuf>,
    pub selected: Option<PathBuf>,
    pub show_excluded: bool,
    /// Acción pendiente al abrir (no se guarda: viene de `vkterm --then`).
    pub then: Option<String>,
}

/// Solo rutas dentro del proyecto: relativas y sin `..`.
fn inside(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    let ok = !path.as_os_str().is_empty() && path.components().all(|c| matches!(c, Component::Normal(_)));
    ok.then(|| path.to_path_buf())
}

impl Memory {
    pub fn parse(text: &str) -> Memory {
        let mut memory = Memory::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            let flag = value == "1";
            match key {
                "sidebar" => memory.visible.sidebar = flag,
                "panel" => memory.visible.panel = flag,
                "agents" => memory.visible.agents = flag,
                "excluded" => memory.show_excluded = flag,
                "focus" if FOCUS_TARGETS.contains(&value) => memory.focus = Some(value.to_string()),
                "expanded" => memory.expanded.extend(inside(value)),
                "selected" => memory.selected = inside(value),
                "then" if THEN_ACTIONS.contains(&value) => memory.then = Some(value.to_string()),
                _ => {}
            }
        }
        memory
    }

    /// Lo que se guarda (sin `then`).
    pub fn to_text(&self) -> String {
        let flag = |b: bool| if b { "1" } else { "0" };
        let mut lines = vec![
            format!("sidebar={}", flag(self.visible.sidebar)),
            format!("panel={}", flag(self.visible.panel)),
            format!("agents={}", flag(self.visible.agents)),
            format!("excluded={}", flag(self.show_excluded)),
        ];
        if let Some(focus) = &self.focus {
            lines.push(format!("focus={focus}"));
        }
        for dir in &self.expanded {
            lines.push(format!("expanded={}", dir.display()));
        }
        if let Some(selected) = &self.selected {
            lines.push(format!("selected={}", selected.display()));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta() {
        let memory = Memory {
            visible: Visible { sidebar: false, panel: true, agents: false },
            focus: Some("shell".into()),
            expanded: vec!["src".into(), "src/components".into()],
            selected: Some("src/App.tsx".into()),
            show_excluded: true,
            then: None,
        };
        assert_eq!(Memory::parse(&memory.to_text()), memory);
    }

    #[test]
    fn sin_memoria_todo_visible() {
        let memory = Memory::parse("");
        assert_eq!(memory.visible, Visible::default());
        assert_eq!(memory.focus, None);
        assert!(memory.expanded.is_empty());
    }

    #[test]
    fn ignora_claves_y_valores_desconocidos() {
        let memory = Memory::parse("focus=terminal\nthen=rm\nfuture=1\nsin igual\nthen=quick_open");
        assert_eq!(memory.focus, None);
        assert_eq!(memory.then.as_deref(), Some("quick_open"));
    }

    #[test]
    fn solo_rutas_dentro_del_proyecto() {
        let memory = Memory::parse("expanded=/etc\nexpanded=../fuera\nexpanded=src/../..\nexpanded=src\nselected=/etc/passwd");
        assert_eq!(memory.expanded, vec![PathBuf::from("src")]);
        assert_eq!(memory.selected, None);
    }

    #[test]
    fn then_no_se_guarda() {
        let memory = Memory { then: Some("quick_open".into()), ..Default::default() };
        assert!(!memory.to_text().contains("then"));
    }
}
