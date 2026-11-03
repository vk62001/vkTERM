//! ⌘K F «Cerrar carpeta», como «File: Close Folder» de VS Code: termina la
//! sesión del IDE y vuelve al modo terminal. Solo pregunta si se perdería
//! algo: archivos sin guardar en el editor o Claude Code / Codex en marcha.
//!
//! El explorador (controlador) reúne lo que se perdería y lo pasa como texto
//! a la ventana de confirmación (src/confirm.rs), otra instancia del plugin.
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Loss {
    /// Nombre de la carpeta, para el texto de la ventana.
    pub project: String,
    /// Agentes en marcha (no los que esperan "Waiting to run").
    pub agents: Vec<String>,
    /// Archivos sin guardar, relativos al proyecto si están dentro.
    pub modified: Vec<String>,
    /// Pestañas sin nombre con texto.
    pub unnamed: usize,
}

/// Lo que se puede elegir en la ventana de confirmación.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Save,
    Discard,
    Close,
    Cancel,
}

impl Choice {
    pub fn label(self) -> &'static str {
        match self {
            Choice::Save => "Guardar y cerrar",
            Choice::Discard => "Cerrar sin guardar",
            Choice::Close => "Cerrar",
            Choice::Cancel => "Cancelar",
        }
    }

    /// Payload de `close_folder_confirmed` (Cancel no manda nada).
    pub fn payload(self) -> Option<&'static str> {
        match self {
            Choice::Save => Some("save"),
            Choice::Discard => Some("discard"),
            Choice::Close => Some("close"),
            Choice::Cancel => None,
        }
    }
}

impl Loss {
    /// Desde el archivo `editor` que escribe el plugin Lua de Micro
    /// (modified=<ruta absoluta>, unnamed=<n>).
    pub fn read_editor(&mut self, editor: &str, project: &Path) {
        for line in editor.lines() {
            match line.split_once('=') {
                Some(("modified", path)) => {
                    let shown = Path::new(path)
                        .strip_prefix(project)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| path.to_string());
                    if !self.modified.contains(&shown) {
                        self.modified.push(shown);
                    }
                }
                Some(("unnamed", n)) => self.unnamed = n.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty() && self.modified.is_empty() && self.unnamed == 0
    }

    pub fn unsaved(&self) -> bool {
        !self.modified.is_empty() || self.unnamed > 0
    }

    /// Botones, el primero el predeterminado. Las pestañas sin nombre no se
    /// pueden guardar sin preguntar dónde: entonces no hay "Guardar y cerrar".
    pub fn choices(&self) -> Vec<Choice> {
        match (self.unsaved(), self.unnamed > 0) {
            (false, _) => vec![Choice::Close, Choice::Cancel],
            (true, false) => vec![Choice::Save, Choice::Discard, Choice::Cancel],
            (true, true) => vec![Choice::Discard, Choice::Cancel],
        }
    }

    /// Texto de la ventana: qué se pierde al cerrar.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!("Se cerrará la carpeta {} y, con ella:", self.project), String::new()];
        for agent in &self.agents {
            lines.push(format!("• {agent}, que está en marcha"));
        }
        match self.modified.len() {
            0 => {}
            1 => lines.push("• 1 archivo sin guardar:".to_string()),
            n => lines.push(format!("• {n} archivos sin guardar:")),
        }
        lines.extend(self.modified.iter().take(6).map(|f| format!("    {f}")));
        if self.modified.len() > 6 {
            lines.push(format!("    y {} más", self.modified.len() - 6));
        }
        match self.unnamed {
            0 => {}
            1 => lines.push("• 1 pestaña sin nombre con cambios (no se puede guardar desde aquí)".to_string()),
            n => lines.push(format!("• {n} pestañas sin nombre con cambios (no se pueden guardar desde aquí)")),
        }
        lines
    }

    /// Para el mensaje a la ventana de confirmación.
    pub fn to_text(&self) -> String {
        let mut lines = vec![format!("project={}", self.project), format!("unnamed={}", self.unnamed)];
        lines.extend(self.agents.iter().map(|a| format!("agent={a}")));
        lines.extend(self.modified.iter().map(|m| format!("modified={m}")));
        lines.join("\n")
    }

    pub fn parse(text: &str) -> Loss {
        let mut loss = Loss::default();
        for line in text.lines() {
            match line.split_once('=') {
                Some(("project", v)) => loss.project = v.to_string(),
                Some(("agent", v)) => loss.agents.push(v.to_string()),
                Some(("modified", v)) => loss.modified.push(v.to_string()),
                Some(("unnamed", v)) => loss.unnamed = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        loss
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EDITOR: &str = "tab=/p/demo/src/App.tsx\ntab=/p/demo/README.md\nactive=/p/demo/README.md\n\
                          modified=/p/demo/src/App.tsx\nmodified=/otra/nota.txt\nunnamed=1\n";

    #[test]
    fn lee_lo_modificado_relativo_al_proyecto() {
        let mut loss = Loss::default();
        loss.read_editor(EDITOR, Path::new("/p/demo"));
        assert_eq!(loss.modified, ["src/App.tsx", "/otra/nota.txt"]);
        assert_eq!(loss.unnamed, 1);
    }

    #[test]
    fn sin_cambios_ni_agentes_no_pregunta() {
        let mut loss = Loss::default();
        loss.read_editor("tab=/p/demo/a.txt\nactive=/p/demo/a.txt\n", Path::new("/p/demo"));
        assert!(loss.is_empty());
    }

    #[test]
    fn botones_segun_lo_que_se_pierde() {
        let agents = Loss { agents: vec!["Claude Code".into()], ..Default::default() };
        assert_eq!(agents.choices(), [Choice::Close, Choice::Cancel]);
        let files = Loss { modified: vec!["a.txt".into()], ..Default::default() };
        assert_eq!(files.choices(), [Choice::Save, Choice::Discard, Choice::Cancel]);
        let unnamed = Loss { modified: vec!["a.txt".into()], unnamed: 1, ..Default::default() };
        assert_eq!(unnamed.choices(), [Choice::Discard, Choice::Cancel]);
    }

    #[test]
    fn ida_y_vuelta_por_el_mensaje() {
        let loss = Loss {
            project: "DEMO".into(),
            agents: vec!["Claude Code".into(), "Codex".into()],
            modified: vec!["src/App.tsx".into()],
            unnamed: 2,
        };
        assert_eq!(Loss::parse(&loss.to_text()), loss);
    }

    #[test]
    fn texto_de_la_ventana() {
        let loss = Loss {
            project: "DEMO".into(),
            agents: vec!["Codex".into()],
            modified: vec!["a.txt".into(), "b.txt".into()],
            unnamed: 1,
        };
        let text = loss.lines().join("\n");
        assert!(text.starts_with("Se cerrará la carpeta DEMO y, con ella:"));
        assert!(text.contains("• Codex, que está en marcha"));
        assert!(text.contains("• 2 archivos sin guardar:\n    a.txt\n    b.txt"));
        assert!(text.contains("1 pestaña sin nombre"));
    }
}
