//! Ventana de confirmación, como los diálogos de VS Code (src/question.rs):
//! - ⌘K F «Cerrar carpeta» con cambios sin guardar: dice qué se perdería
//!   (archivos sin guardar, Claude Code o Codex en marcha) y ofrece guardar,
//!   cerrar sin guardar o cancelar (src/closing.rs);
//! - la vista Control de código fuente: descartar cambios, commit sin nada
//!   preparado, abortar una fusión.
//!
//! Es otra instancia del plugin (configuración `mode "confirm"`), como la
//! paleta. El explorador (controlador) la abre con el mensaje `close_folder`
//! y lo que se perdería como payload, o con `ask` y la pregunta. Lo elegido
//! vuelve al controlador en un mensaje `palette`, que lo ejecuta cuando la
//! ventana ya se ha cerrado. Esc, un clic fuera o perder el foco cancelan.
use crate::floating::{self, Floating, FRAME};
use vkterm::closing::Loss;
use vkterm::question::Question;
use zellij_tile::prelude::*;

const HINT: &str = "← → elegir · Enter aceptar · Esc cancelar";
const PAD_X: usize = 2;
const PAD_Y: usize = 1;
const BUTTON_GAP: usize = 2;

#[derive(Default)]
pub struct Confirm {
    floating: Floating,
    controller: Option<u32>,
    question: Question,
    selected: usize,
    /// Columnas de cada botón en la fila de botones, para los clics.
    buttons: Vec<(usize, usize)>,
}

impl Confirm {
    pub fn load(&mut self) {
        self.floating.load(&[]);
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        let payload = message.payload.unwrap_or_default();
        let question = match message.name.as_str() {
            "close_folder" => Question::close_folder(&Loss::parse(&payload)),
            "ask" => Question::parse(&payload),
            _ => return false,
        };
        if self.floating.open(&question.title) {
            self.question = question;
            self.selected = 0;
        }
        true
    }

    pub fn update(&mut self, event: Event) -> bool {
        if self.floating.lost_focus(&event) {
            close_self();
            return false;
        }
        if self.floating.theme(&event) {
            return true;
        }
        match event {
            Event::PaneUpdate(manifest) => {
                self.controller = floating::controller(&manifest, self.floating.me);
                let lines = &self.question.lines;
                let content = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0).max(self.buttons_width());
                let width = content.max(HINT.chars().count() + 2) + 2 * (FRAME + PAD_X);
                self.floating.place(width, lines.len() + 2 + 2 * (FRAME + PAD_Y));
                false
            }
            Event::Key(key) => {
                let count = self.question.buttons.len().max(1);
                match key.bare_key {
                    BareKey::Esc => close_self(),
                    BareKey::Enter if key.has_no_modifiers() => self.choose(self.selected),
                    BareKey::Left => self.selected = (self.selected + count - 1) % count,
                    BareKey::Right | BareKey::Tab => self.selected = (self.selected + 1) % count,
                    _ => return false,
                }
                true
            }
            Event::Mouse(Mouse::LeftClick(line, column)) => {
                let row = FRAME + PAD_Y + self.question.lines.len() + 1;
                let x = column.saturating_sub(FRAME + PAD_X);
                match self.buttons.iter().position(|(start, end)| line as usize == row && (*start..*end).contains(&x)) {
                    Some(i) => self.choose(i),
                    None => close_self(),
                }
                true
            }
            _ => false,
        }
    }

    fn buttons_width(&self) -> usize {
        self.question.buttons.iter().map(|(label, _)| label.chars().count() + 4 + BUTTON_GAP).sum::<usize>()
    }

    fn choose(&mut self, i: usize) {
        if let (Some(answer), Some(controller)) = (self.question.answer(i), self.controller) {
            pipe_message_to_plugin(
                MessageToPlugin::new("palette").with_destination_plugin_id(controller).with_payload(answer),
            );
        }
        close_self();
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let palette = self.floating.palette;
        floating::frame(rows, cols, &self.question.title, HINT, &palette);
        let x = FRAME + PAD_X;
        let width = cols.saturating_sub(2 * x);
        let lines = &self.question.lines;
        for (y, text) in lines.iter().enumerate() {
            let mut line = floating::row(text, width, &palette);
            if y == 0 {
                line.paint(0..width, |l| l.bold());
            }
            print!("{}", line.render(x, FRAME + PAD_Y + y));
        }
        // Botones: "[ Etiqueta ]", el elegido resaltado como la selección.
        self.buttons.clear();
        let mut start = 0;
        for (i, (label, _)) in self.question.buttons.iter().enumerate() {
            let label = format!("[ {label} ]");
            let len = label.chars().count();
            let mut button = floating::row(&label, len, &palette);
            if i == self.selected {
                button.paint(0..len, |_| palette.quick_selection.bold());
            } else {
                button.fg(0..len, palette.quick_description);
            }
            print!("{}", button.render(x + start, FRAME + PAD_Y + lines.len() + 1));
            self.buttons.push((start, start + len));
            start += len + BUTTON_GAP;
        }
    }
}
