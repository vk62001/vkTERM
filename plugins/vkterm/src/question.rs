//! Preguntas de la ventana de confirmación (src/confirm.rs), como los
//! diálogos de VS Code: un título, unas líneas de texto y botones, el primero
//! el predeterminado. Las usan ⌘K F «Cerrar carpeta» (src/closing.rs) y la
//! vista Control de código fuente (descartar cambios, commit sin nada
//! preparado, abortar una fusión).
//!
//! El controlador la manda como texto a la ventana; el botón elegido vuelve en
//! un mensaje `palette` con "<message>\t<payload>". Cancelar no manda nada.
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.
use crate::closing::Loss;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Question {
    pub title: String,
    pub lines: Vec<String>,
    /// Mensaje al controlador con el payload del botón elegido.
    pub message: String,
    /// (etiqueta, payload); sin payload, cancela.
    pub buttons: Vec<(String, Option<String>)>,
}

impl Question {
    /// Una pregunta con un botón que acepta (`payload`) y «Cancelar».
    pub fn confirm(title: &str, lines: &[String], accept: &str, message: &str, payload: &str) -> Question {
        Question {
            title: title.to_string(),
            lines: lines.to_vec(),
            message: message.to_string(),
            buttons: vec![(accept.to_string(), Some(payload.to_string())), ("Cancelar".to_string(), None)],
        }
    }

    /// La de ⌘K F, con lo que se perdería.
    pub fn close_folder(loss: &Loss) -> Question {
        Question {
            title: "Cerrar carpeta".to_string(),
            lines: loss.lines(),
            message: "close_folder_confirmed".to_string(),
            buttons: loss.choices().iter().map(|c| (c.label().to_string(), c.payload().map(str::to_string))).collect(),
        }
    }

    /// Líneas clave=valor (los textos no tienen saltos de línea).
    pub fn to_text(&self) -> String {
        let one = |s: &str| s.replace('\n', " ");
        let mut lines = vec![format!("title={}", one(&self.title)), format!("message={}", one(&self.message))];
        lines.extend(self.lines.iter().map(|l| format!("line={}", one(l))));
        for (label, payload) in &self.buttons {
            lines.push(format!("button={}\t{}", one(label), payload.as_deref().map(one).unwrap_or_default()));
        }
        lines.join("\n")
    }

    pub fn parse(text: &str) -> Question {
        let mut question = Question::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            match key {
                "title" => question.title = value.to_string(),
                "message" => question.message = value.to_string(),
                "line" => question.lines.push(value.to_string()),
                "button" => {
                    let (label, payload) = value.split_once('\t').unwrap_or((value, ""));
                    let payload = (!payload.is_empty()).then(|| payload.to_string());
                    question.buttons.push((label.to_string(), payload));
                }
                _ => {}
            }
        }
        question
    }

    /// Lo que se manda al controlador al elegir el botón `i`.
    pub fn answer(&self, i: usize) -> Option<String> {
        let payload = self.buttons.get(i)?.1.as_ref()?;
        Some(format!("{}\t{payload}", self.message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta() {
        let q = Question::confirm("Descartar cambios", &["¿Descartar a.txt?".into(), "".into()], "Descartar", "scm_answer", "yes");
        let back = Question::parse(&q.to_text());
        assert_eq!(back, q);
        assert_eq!(back.answer(0).as_deref(), Some("scm_answer\tyes"));
        assert_eq!(back.answer(1), None);
    }

    #[test]
    fn la_de_cerrar_carpeta_sale_de_lo_que_se_perderia() {
        let loss = Loss { project: "demo".into(), agents: vec!["Codex".into()], ..Default::default() };
        let q = Question::parse(&Question::close_folder(&loss).to_text());
        assert_eq!(q.title, "Cerrar carpeta");
        assert_eq!(q.lines, loss.lines());
        assert_eq!(q.buttons.iter().map(|b| b.0.as_str()).collect::<Vec<_>>(), ["Cerrar", "Cancelar"]);
        assert_eq!(q.answer(0).as_deref(), Some("close_folder_confirmed\tclose"));
    }

    #[test]
    fn sin_saltos_de_linea_en_los_textos() {
        let q = Question::confirm("T", &["a\nb".into()], "Sí", "m", "p");
        assert_eq!(Question::parse(&q.to_text()).lines, ["a b"]);
    }
}
