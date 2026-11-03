//! Ventana «Nuevo archivo» (⌘N) y «Renombrar» (⌘R, con el nombre actual ya
//! escrito; mensaje `rename_file` y `rename_to` de vuelta): pide el nombre, como el «Nuevo archivo…»
//! del explorador de VS Code, y dice debajo dónde se creará o por qué no se
//! puede (src/newfile.rs). Enter lo crea; Esc, un clic fuera o perder el
//! foco cancelan.
//!
//! Es otra instancia del plugin (configuración `mode "newfile"`), como la
//! paleta. El explorador (controlador) la abre con el mensaje `new_file` y la
//! carpeta seleccionada como payload ("" es la raíz). El archivo elegido
//! vuelve al controlador en un mensaje `palette` (`create_file`), que lo crea,
//! lo muestra en el explorador y lo abre en el editor cuando la ventana ya se
//! ha cerrado.
use std::path::{Path, PathBuf};

use crate::floating::{self, Floating, FRAME};
use vkterm::newfile::{self, Problem};
use zellij_tile::prelude::*;

pub const TITLE: &str = "Nuevo archivo";
const RENAME_TITLE: &str = "Renombrar";
const HINT: &str = "Enter crear · Esc cancelar";
const RENAME_HINT: &str = "Enter renombrar · Esc cancelar";
const PAD_X: usize = 2;

#[derive(Default)]
pub struct Prompt {
    floating: Floating,
    controller: Option<u32>,
    folder: PathBuf,
    name: String,
    /// Con `rename_file`: el archivo o carpeta que se renombra.
    renaming: Option<PathBuf>,
}

impl Prompt {
    pub fn load(&mut self) {
        self.floating.load(&[]);
    }

    pub fn pipe(&mut self, message: PipeMessage) -> bool {
        let renaming = match message.name.as_str() {
            "new_file" => false,
            "rename_file" => true,
            _ => return false,
        };
        let title = if renaming { RENAME_TITLE } else { TITLE };
        if self.floating.open(title) {
            let payload = message.payload.unwrap_or_default();
            if renaming {
                let old = PathBuf::from(payload.trim());
                self.name = old.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                self.folder = old.parent().map(Path::to_path_buf).unwrap_or_default();
                self.renaming = Some(old);
            } else {
                self.folder = PathBuf::from(payload.trim());
                self.name.clear();
                self.renaming = None;
            }
        } else {
            // ⌘N (o ⌘R) otra vez la cierra, como las demás ventanas.
            close_self();
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
                let (columns, _) = self.floating.area();
                self.floating.place((columns * 3 / 5).clamp(44, 100), 2 + 2 * FRAME);
                false
            }
            Event::Key(key) => {
                let typed = key.has_no_modifiers() || key.has_only_modifiers(&[KeyModifier::Shift]);
                match key.bare_key {
                    BareKey::Esc => close_self(),
                    BareKey::Enter if key.has_no_modifiers() => self.create(),
                    BareKey::Backspace => {
                        self.name.pop();
                    }
                    BareKey::Char('u') if key.has_only_modifiers(&[KeyModifier::Ctrl]) => self.name.clear(),
                    BareKey::Char(c) if typed && !c.is_control() => self.name.push(c),
                    _ => return false,
                }
                true
            }
            _ => false,
        }
    }

    fn target(&self) -> Result<PathBuf, Problem> {
        let kind = |path: &Path| {
            let host = Path::new(crate::HOST).join(path);
            host.symlink_metadata().ok().map(|_| host.is_dir())
        };
        match &self.renaming {
            Some(old) => newfile::rename_target(old, &self.name, kind),
            None => newfile::target(&self.folder, &self.name, kind),
        }
    }

    /// Enter: si se puede, manda el archivo al controlador y se cierra.
    fn create(&mut self) {
        let (Ok(file), Some(controller)) = (self.target(), self.controller) else {
            return;
        };
        let payload = match &self.renaming {
            Some(old) => format!("rename_to\t{}\t{}", old.display(), file.display()),
            None => format!("create_file\t{}", file.display()),
        };
        pipe_message_to_plugin(
            MessageToPlugin::new("palette").with_destination_plugin_id(controller).with_payload(payload),
        );
        close_self();
    }

    pub fn render(&mut self, rows: usize, cols: usize) {
        let palette = self.floating.palette;
        let (title, hint) = if self.renaming.is_some() { (RENAME_TITLE, RENAME_HINT) } else { (TITLE, HINT) };
        floating::frame(rows, cols, title, hint, &palette);
        let x = FRAME + PAD_X;
        let width = cols.saturating_sub(2 * x);
        if width < 8 || rows < 2 * FRAME + 2 {
            return;
        }
        let placeholder = "Nombre del archivo (con / crea carpetas)";
        let renamed = self.renaming.as_deref();
        print!("{}", floating::input_line("›", &self.name, placeholder, width, &palette).render(x, FRAME));
        let (text, color) = match self.target() {
            Ok(file) if renamed.is_some() => (format!("Pasará a {}", file.display()), palette.quick_description),
            Ok(file) => (format!("Se creará {}", file.display()), palette.quick_description),
            Err(Problem::Empty) => (format!("En {}", folder_label(&self.folder)), palette.quick_description),
            Err(problem) => (problem.message(&self.name), palette.error),
        };
        let mut line = floating::row(&text, width, &palette);
        line.fg(0..width, color);
        print!("{}", line.render(x, FRAME + 1));
    }
}

/// La carpeta de destino, para leerla: "src/components/" o la raíz.
fn folder_label(folder: &Path) -> String {
    if folder.as_os_str().is_empty() {
        "la raíz del proyecto".to_string()
    } else {
        format!("{}/", folder.display())
    }
}
