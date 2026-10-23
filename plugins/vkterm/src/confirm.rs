//! Confirmación de ⌘K F «Cerrar carpeta», como el diálogo de VS Code al cerrar
//! con cambios sin guardar: dice qué se perdería (archivos sin guardar,
//! Claude Code o Codex en marcha) y ofrece guardar, cerrar sin guardar o
//! cancelar (src/closing.rs).
//!
//! Es otra instancia del plugin (configuración `mode "confirm"`), como la
//! paleta. El explorador (controlador) la abre con el mensaje `close_folder`
//! y lo que se perdería como payload. Lo elegido vuelve al controlador en un
//! mensaje `palette` (`close_folder_confirmed`), que lo ejecuta cuando la
//! ventana ya se ha cerrado. Esc, un clic fuera o perder el foco cancelan.
use crate::floating::{self, Floating, FRAME};
use vkterm::closing::{Choice, Loss};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Cerrar carpeta";
const HINT: &str = "← → elegir · Enter aceptar · Esc cancelar";
const PAD_X: usize = 2;
const PAD_Y: usize = 1;
const BUTTON_GAP: usize = 2;

#[derive(Default)]
pub struct Confirm {
    floating: Floating,
    controller: Option<u32>,
    loss: Loss,
    choices: Vec<Choice>,
    selected: usize,
    /// Columnas de cada botón en la fila de botones, para los clics.
    buttons: Vec<(usize, usize)>,
}

impl Confirm {
    pub fn load(&mut self) {
        self.floating.load(&[]);
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        if message.name != "close_folder" {
            return false;
        }
        if self.floating.open(TITLE) {
            self.loss = Loss::parse(&message.payload.unwrap_or_default());
            self.choices = self.loss.choices();
            self.selected = 0;
        }
        true
    }

    pub fn update(&mut self, event: Event) -> bool {
        if self.floating.lost_focus(&event) {
            close_self();
            return false;
        }
        match event {
            Event::PaneUpdate(manifest) => {
                self.controller = floating::controller(&manifest, self.floating.me);
                let lines = self.loss.lines();
                let content = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0).max(self.buttons_width());
                let width = content.max(HINT.chars().count() + 2) + 2 * (FRAME + PAD_X);
                self.floating.place(width, lines.len() + 2 + 2 * (FRAME + PAD_Y));
                false
            }
            Event::Key(key) => {
                let count = self.choices.len().max(1);
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
                let row = FRAME + PAD_Y + self.loss.lines().len() + 1;
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
        self.choices.iter().map(|c| c.label().chars().count() + 4 + BUTTON_GAP).sum::<usize>()
    }

    fn choose(&mut self, i: usize) {
        if let (Some(payload), Some(controller)) = (self.choices.get(i).and_then(|c| c.payload()), self.controller) {
            pipe_message_to_plugin(
                MessageToPlugin::new("palette")
                    .with_destination_plugin_id(controller)
                    .with_payload(format!("close_folder_confirmed\t{payload}")),
            );
        }
        close_self();
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let x = FRAME + PAD_X;
        let width = cols.saturating_sub(2 * x);
        let lines = self.loss.lines();
        for (y, line) in lines.iter().enumerate() {
            let shown = crate::fit(line, width);
            let mut text = Text::new(&shown);
            if y == 0 {
                text = text.color_range(2, ..);
            }
            print_text_with_coordinates(text, x, FRAME + PAD_Y + y, Some(width), Some(1));
        }
        // Botones: "[ Etiqueta ]", el elegido resaltado.
        self.buttons.clear();
        let mut start = 0;
        for (i, choice) in self.choices.iter().enumerate() {
            let label = format!("[ {} ]", choice.label());
            let len = label.chars().count();
            let mut text = Text::new(&label);
            text = if i == self.selected { text.selected().color_range(3, ..) } else { text.dim_all() };
            print_text_with_coordinates(text, x + start, FRAME + PAD_Y + lines.len() + 1, None, None);
            self.buttons.push((start, start + len));
            start += len + BUTTON_GAP;
        }
        floating::frame(rows, cols, TITLE, HINT);
    }
}
