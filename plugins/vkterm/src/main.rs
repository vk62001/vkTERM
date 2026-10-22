//! vkterm: explorador tipo VS Code y controlador del workspace de vkTERM.
//!
//! - **Explorador**: muestra el proyecto como árbol en la barra lateral. Abre
//!   los archivos como pestañas del Micro del panel Editor (bin/vkterm-open).
//!   Nunca abre terminales.
//! - **Controlador**: recibe los atajos (`MessagePlugin "vkterm"` en
//!   config.kdl). Oculta y muestra la barra lateral, el panel (Shell) y los
//!   agentes, y enfoca paneles por identidad. Sigue recibiendo atajos aunque
//!   su panel esté oculto.
//! - **Foco**: como "Dim inactive split panes" de iTerm2, los paneles sin foco
//!   se ven atenuados y el enfocado con sus colores normales.
//!
//! Con `mode "shortcuts"` en la configuración es, en cambio, la ventana de
//! atajos (src/overlay.rs); con `mode "themes"`, el selector de tema
//! (src/picker.rs); con `mode "palette"`, la paleta de archivos y comandos
//! (src/palette.rs). Con otro `mode`, se cierra.
mod floating;
mod overlay;
mod palette;
mod picker;

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use overlay::Overlay;
use palette::Palette;
use picker::Picker;
use vkterm::clipboard;
use vkterm::commands;
use vkterm::layout::{self, Visible};
use vkterm::themes;
use vkterm::tree::{Action, DirEntry, Tree};
use zellij_tile::prelude::*;

/// Carpeta del proyecto vista desde el plugin.
pub(crate) const HOST: &str = "/host";
const HEADER_ROWS: usize = 1;
const FOOTER_ROWS: usize = 1;
const HINT: &str = "Enter abrir · Espacio vista previa · ⌘B ocultar";
/// Permisos de todas las instancias (explorador, atajos y temas). Reconfigure
/// cambia el tema de Zellij; MessageAndLaunchOtherPlugins avisa al explorador
/// del tema elegido.
const PERMISSIONS: [PermissionType; 6] = [
    PermissionType::ReadApplicationState,
    PermissionType::ChangeApplicationState,
    PermissionType::WriteToStdin,
    PermissionType::RunCommands,
    PermissionType::Reconfigure,
    PermissionType::MessageAndLaunchOtherPlugins,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Zone {
    Sidebar,
    Panel,
    Agents,
}

#[derive(Default)]
struct State {
    tree: Tree,
    visible: Visible,
    /// Paneles de la pestaña del workspace (incluidos los ocultos).
    panes: Vec<PaneInfo>,
    my_tab: usize,
    active_tab: usize,
    me: u32,
    project: PathBuf,
    session: Option<String>,
    status: Option<String>,
    rows: usize,
    editor_restarting: bool,
    /// Hay paneles flotantes a la vista en la pestaña del workspace (la
    /// ventana de atajos). Zellij oculta esa capa al enfocar un panel fijo:
    /// mientras se ve, el foco está en ella.
    floating_active: bool,
    /// Colores (texto, fondo) de los paneles sin foco, "#rrggbb": los del
    /// tema, salvo que config.kdl defina inactive_fg e inactive_bg.
    inactive: Option<(String, String)>,
    colors_override: Option<((u8, u8, u8), (u8, u8, u8))>,
    /// Los mismos colores en SGR, para pintar el explorador sin foco.
    inactive_sgr: String,
    /// Ya se preguntó a bin/vkterm-theme por el tema guardado.
    asked_theme: bool,
    /// Esta instancia es una ventana flotante en lugar del explorador.
    window: Option<Window>,
    /// Lo elegido en la paleta, pendiente de que su ventana se cierre y el
    /// foco vuelva a los paneles fijos: (mensaje, payload).
    pending: Option<(String, String)>,
}

enum Window {
    Shortcuts(Overlay),
    Themes(Picker),
    Palette(Palette),
    /// `mode` desconocido: se cierra sin hacer nada.
    Closed,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        match configuration.get("mode").map(String::as_str) {
            Some("shortcuts") => {
                let mut overlay = Overlay::default();
                overlay.load();
                self.window = Some(Window::Shortcuts(overlay));
                return;
            }
            Some("themes") => {
                let mut picker = Picker::default();
                picker.load();
                self.window = Some(Window::Themes(picker));
                return;
            }
            Some("palette") => {
                let mut palette = Palette::default();
                palette.load();
                self.window = Some(Window::Palette(palette));
                return;
            }
            // Una ventana que esta versión no conoce (un atajo de una
            // configuración más nueva): sin esto sería un segundo explorador
            // y controlador.
            Some(_) => {
                self.window = Some(Window::Closed);
                close_self();
                return;
            }
            None => {}
        }
        let color = |key: &str| configuration.get(key).and_then(|value| rgb(value));
        self.colors_override = color("inactive_fg").zip(color("inactive_bg"));
        self.set_theme(themes::DEFAULT);
        request_permission(&PERMISSIONS);
        subscribe(&[
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::ModeUpdate,
            EventType::Key,
            EventType::Mouse,
            EventType::FileSystemCreate,
            EventType::FileSystemUpdate,
            EventType::FileSystemDelete,
            EventType::RunCommandResult,
        ]);
        set_selectable(true);
        let ids = get_plugin_ids();
        self.me = ids.plugin_id;
        self.project = ids.initial_cwd;
        self.load_dir(Path::new(""));
        watch_filesystem();
    }

    fn update(&mut self, event: Event) -> bool {
        match &mut self.window {
            Some(Window::Shortcuts(overlay)) => return overlay.update(event),
            Some(Window::Themes(picker)) => return picker.update(event),
            Some(Window::Palette(palette)) => return palette.update(event),
            Some(Window::Closed) => return false,
            None => {}
        }
        match event {
            Event::PaneUpdate(manifest) => {
                for (tab, panes) in manifest.panes {
                    if panes.iter().any(|p| p.is_plugin && p.id == self.me) {
                        self.my_tab = tab;
                        self.panes = panes;
                    }
                }
                self.restart_editor_if_closed();
                self.dim_inactive();
                true
            }
            Event::TabUpdate(tabs) => {
                if let Some(tab) = tabs.iter().find(|t| t.active) {
                    self.active_tab = tab.position;
                }
                let floating = tabs.iter().any(|t| t.position == self.my_tab && t.are_floating_panes_visible);
                if !floating {
                    if let Some((name, payload)) = self.pending.take() {
                        self.dispatch(&name, &payload);
                    }
                }
                if floating == self.floating_active {
                    return false;
                }
                self.floating_active = floating;
                self.dim_inactive();
                true
            }
            Event::ModeUpdate(mode) => {
                self.session = mode.session_name;
                // Llega ya con los permisos concedidos.
                if !self.asked_theme {
                    self.asked_theme = true;
                    let context = BTreeMap::from([("vkterm".to_string(), "current".to_string())]);
                    run_command(&["vkterm-theme", "current"], context);
                }
                false
            }
            Event::RunCommandResult(_, stdout, _, context)
                if context.get("vkterm").map(String::as_str) == Some("current") =>
            {
                self.set_theme(&String::from_utf8_lossy(&stdout));
                true
            }
            Event::RunCommandResult(_, stdout, _, context)
                if context.get("vkterm").map(String::as_str) == Some("paste") =>
            {
                let pane = context.get("pane").and_then(|id| id.parse().ok());
                if let Some(pane) = pane.filter(|_| clipboard::image_only(&String::from_utf8_lossy(&stdout))) {
                    write_to_pane_id(vec![0x16], PaneId::Terminal(pane));
                }
                false
            }
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(mouse) => self.handle_mouse(mouse),
            Event::FileSystemCreate(paths)
            | Event::FileSystemUpdate(paths)
            | Event::FileSystemDelete(paths) => {
                self.refresh(paths.into_iter().map(|(p, _)| p).collect());
                true
            }
            _ => false,
        }
    }

    fn pipe(&mut self, message: PipeMessage) -> bool {
        match &mut self.window {
            Some(Window::Shortcuts(overlay)) => return overlay.pipe(message),
            Some(Window::Themes(picker)) => return picker.pipe(message),
            Some(Window::Palette(palette)) => return palette.pipe(message),
            Some(Window::Closed) => return false,
            None => {}
        }
        let payload = message.payload.unwrap_or_default();
        if message.name == "palette" {
            // De la paleta (src/palette.rs): se ejecuta cuando su ventana ya
            // se ha cerrado, para que el foco y los paneles estén en su sitio.
            let (name, payload) = payload.split_once('\t').unwrap_or((&payload, ""));
            if self.floating_active {
                self.pending = Some((name.to_string(), payload.to_string()));
            } else {
                self.dispatch(name, payload);
            }
            return false;
        }
        self.dispatch(&message.name, &payload)
    }

    fn render(&mut self, rows: usize, cols: usize) {
        match &mut self.window {
            Some(Window::Shortcuts(overlay)) => return overlay.render(rows, cols),
            Some(Window::Themes(picker)) => return picker.render(rows, cols),
            Some(Window::Palette(palette)) => return palette.render(rows, cols),
            Some(Window::Closed) => return,
            None => {}
        }
        self.rows = rows;
        // Las líneas con fondo de color no se recortan al estrechar el panel
        // (al volver a mostrarlo con ⌘B): Zellij las parte y las pasa al
        // scroll. Cada render vacía el panel, pero el marco seguiría mostrando
        // "SCROLL: 0/N"; CSI 3 J pone el contador a cero.
        print!("\u{1b}[3J");
        // Sin foco, todo el panel con los colores de los terminales sin foco
        // (dim_inactive); la selección se sigue viendo con el color del tema.
        let dim = !self.panes.iter().any(|p| p.is_plugin && p.id == self.me && self.has_focus(p));
        let paint = |y: usize, line: &str, text: Text| {
            if dim {
                print!("\u{1b}[{};1H{}{}\u{1b}[0m", y + 1, self.inactive_sgr, line);
            } else {
                print_text_with_coordinates(text, 0, y, Some(cols), Some(1));
            }
        };
        let name = self
            .project
            .file_name()
            .map(|n| n.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        let header = fit(&format!(" {}", name), cols);
        paint(0, &header, Text::new(&header).color_range(0, ..));

        let list = self.tree.rows();
        let selected = self.tree.selected_index(&list);
        for y in 0..self.list_height() {
            let i = self.tree.scroll + y;
            let Some(row) = list.get(i) else {
                let blank = " ".repeat(cols);
                paint(HEADER_ROWS + y, &blank, Text::new(&blank));
                continue;
            };
            let icon = match (row.is_dir, row.expanded) {
                (true, true) => "▾ ",
                (true, false) => "▸ ",
                _ => "  ",
            };
            let prefix = format!(" {}{}", "  ".repeat(row.depth), icon);
            let line = fit(&format!("{}{}", prefix, row.name), cols);
            let start = prefix.chars().count();
            let mut text = Text::new(&line);
            if row.is_dir && start < line.chars().count() {
                text = text.color_range(2, start..line.chars().count());
            }
            if i == selected && dim {
                print_text_with_coordinates(text.selected().dim_all(), 0, HEADER_ROWS + y, Some(cols), Some(1));
                continue;
            }
            if i == selected {
                text = text.selected();
            }
            paint(HEADER_ROWS + y, &line, text);
        }

        let footer = fit(&format!(" {}", self.status.as_deref().unwrap_or(HINT)), cols);
        let text = match self.status {
            Some(_) => Text::new(&footer).error_color_all(),
            None => Text::new(&footer).dim_all(),
        };
        paint(rows.saturating_sub(1), &footer, text);
    }
}

impl State {
    /// Los atajos ⌘ (config.kdl) y los comandos de la paleta.
    fn dispatch(&mut self, name: &str, payload: &str) -> bool {
        match name {
            "toggle_sidebar" => self.toggle(Zone::Sidebar),
            "toggle_panel" => self.toggle(Zone::Panel),
            "toggle_agents" => self.toggle(Zone::Agents),
            "focus" => self.focus(payload.trim()),
            "prev_tab" => self.send_to_editor(b"\x1b,", true),
            "next_tab" => self.send_to_editor(b"\x1b.", true),
            "close_tab" => self.close_tab(),
            "edit" => self.edit(payload.trim()),
            "paste" => self.paste_image(),
            // De la paleta (src/commands.rs).
            "open" => {
                self.status = None;
                self.open(Path::new(payload.trim()), true);
            }
            "editor_key" => {
                if let Some(bytes) = commands::editor_key(payload.trim()) {
                    self.send_to_editor(bytes, true);
                }
            }
            "zoom" => toggle_focus_fullscreen(),
            "toggle_excluded" => self.tree.show_excluded = !self.tree.show_excluded,
            "open_window" => self.open_window(payload.trim()),
            // Del selector de tema (src/picker.rs).
            "set_theme" => self.set_theme(payload),
            _ => return false,
        }
        true
    }

    // ------------------------------------------------------------ explorador

    fn list_height(&self) -> usize {
        self.rows.saturating_sub(HEADER_ROWS + FOOTER_ROWS)
    }

    fn load_dir(&mut self, dir: &Path) {
        let host_dir = Path::new(HOST).join(dir);
        match std::fs::read_dir(&host_dir) {
            Ok(read) => {
                let entries = read
                    .filter_map(Result::ok)
                    .map(|e| DirEntry {
                        name: e.file_name().to_string_lossy().into_owned(),
                        is_dir: e.path().is_dir(),
                    })
                    .collect();
                self.tree.set_listing(dir, entries);
            }
            Err(_) => self.tree.forget(dir),
        }
    }

    /// Vuelve a leer las carpetas afectadas por cambios en el disco.
    fn refresh(&mut self, paths: Vec<PathBuf>) {
        let mut dirs: Vec<PathBuf> = paths
            .iter()
            .filter_map(|p| self.relative(p))
            .filter_map(|p| p.parent().map(Path::to_path_buf))
            .filter(|d| self.tree.is_loaded(d))
            .collect();
        dirs.sort();
        dirs.dedup();
        for dir in dirs {
            self.load_dir(&dir);
        }
    }

    fn relative(&self, path: &Path) -> Option<PathBuf> {
        path.strip_prefix(HOST)
            .or_else(|_| path.strip_prefix(&self.project))
            .ok()
            .map(Path::to_path_buf)
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        let plain = key.has_no_modifiers();
        let page = self.list_height().max(1) as isize;
        let mut focus = true;
        let action = match key.bare_key {
            BareKey::Down if plain => self.moved(1),
            BareKey::Up if plain => self.moved(-1),
            BareKey::PageDown if plain => self.moved(page),
            BareKey::PageUp if plain => self.moved(-page),
            BareKey::Home if plain => {
                self.tree.home();
                Action::None
            }
            BareKey::End if plain => {
                self.tree.end();
                Action::None
            }
            BareKey::Right if plain => self.tree.right(),
            BareKey::Left if plain => {
                self.tree.left();
                Action::None
            }
            BareKey::Enter if plain => self.tree.activate(),
            BareKey::Char(' ') if plain => {
                focus = false;
                self.tree.activate()
            }
            BareKey::Char('h') if key.has_only_modifiers(&[KeyModifier::Ctrl]) => {
                self.tree.show_excluded = !self.tree.show_excluded;
                Action::None
            }
            BareKey::Esc => Action::None,
            BareKey::Char(c) if plain || key.has_only_modifiers(&[KeyModifier::Shift]) => {
                self.tree.jump_to(c);
                Action::None
            }
            _ => return false,
        };
        self.status = None;
        self.run(action, focus);
        self.tree.keep_visible(self.list_height());
        true
    }

    fn moved(&mut self, delta: isize) -> Action {
        self.tree.move_by(delta);
        Action::None
    }

    fn handle_mouse(&mut self, mouse: Mouse) -> bool {
        match mouse {
            Mouse::LeftClick(line, _) if line >= HEADER_ROWS as isize => {
                let index = line as usize - HEADER_ROWS + self.tree.scroll;
                let action = self.tree.click(index);
                self.status = None;
                self.run(action, true);
            }
            Mouse::ScrollUp(n) => self.tree.scroll_by(-(n as isize), self.list_height()),
            Mouse::ScrollDown(n) => self.tree.scroll_by(n as isize, self.list_height()),
            _ => return false,
        }
        true
    }

    fn run(&mut self, action: Action, focus: bool) {
        match action {
            Action::Load(dir) => self.load_dir(&dir),
            Action::Open(file) => self.open(&file, focus),
            Action::None => {}
        }
    }

    /// Abre el archivo en una pestaña del editor central (bin/vkterm-open).
    fn open(&mut self, file: &Path, focus: bool) {
        if is_binary(&Path::new(HOST).join(file)) {
            let name = file.file_name().unwrap_or_default().to_string_lossy();
            self.status = Some(format!("{name}: binario, no se abre"));
            return;
        }
        let mut env = BTreeMap::new();
        if let Some(session) = &self.session {
            env.insert("ZELLIJ_SESSION_NAME".to_string(), session.clone());
        }
        if !focus {
            env.insert("VKTERM_NO_FOCUS".to_string(), "1".to_string());
        }
        let path = self.project.join(file);
        run_command_with_env_variables_and_cwd(
            &["vkterm-open", &path.to_string_lossy()],
            env,
            self.project.clone(),
            BTreeMap::new(),
        );
    }

    // ------------------------------------------------------------ controlador

    fn terminal(&self, title: &str) -> Option<PaneId> {
        self.panes
            .iter()
            .find(|p| !p.is_plugin && p.title == title)
            .map(|p| PaneId::Terminal(p.id))
    }

    fn zone_panes(&self, zone: Zone) -> Vec<PaneId> {
        match zone {
            Zone::Sidebar => vec![PaneId::Plugin(self.me)],
            Zone::Panel => self.terminal("Shell").into_iter().collect(),
            Zone::Agents => ["Claude Code", "Codex"].iter().filter_map(|t| self.terminal(t)).collect(),
        }
    }

    fn is_visible(&self, zone: Zone) -> bool {
        match zone {
            Zone::Sidebar => self.visible.sidebar,
            Zone::Panel => self.visible.panel,
            Zone::Agents => self.visible.agents,
        }
    }

    /// `is_focused` es por capa: con la ventana de atajos a la vista, el panel
    /// fijo que tenía el foco lo sigue marcando.
    fn has_focus(&self, pane: &PaneInfo) -> bool {
        pane.is_focused && !pane.is_suppressed && pane.is_floating == self.floating_active
    }

    fn focused_in(&self, ids: &[PaneId]) -> bool {
        self.panes.iter().any(|p| {
            let id = if p.is_plugin { PaneId::Plugin(p.id) } else { PaneId::Terminal(p.id) };
            self.has_focus(p) && ids.contains(&id)
        })
    }

    fn toggle(&mut self, zone: Zone) {
        let show = !self.is_visible(zone);
        // Como VS Code: ⌘J lleva el foco al terminal; ⌘B y ⌥⌘B no lo mueven.
        self.set_zone(zone, show, zone == Zone::Panel);
    }

    fn set_zone(&mut self, zone: Zone, show: bool, focus: bool) {
        // Con otra pestaña de Zellij activa, el layout se aplicaría a esa.
        if self.active_tab != self.my_tab {
            return;
        }
        let ids = self.zone_panes(zone);
        if ids.is_empty() {
            return;
        }
        if show {
            for (i, id) in ids.iter().enumerate() {
                show_pane_with_id(*id, false, focus && i == 0);
            }
        } else {
            if self.focused_in(&ids) {
                if let Some(editor) = self.terminal("Editor") {
                    focus_pane_with_id(editor, false, false);
                }
            }
            for id in &ids {
                hide_pane_with_id(*id);
            }
        }
        match zone {
            Zone::Sidebar => self.visible.sidebar = show,
            Zone::Panel => self.visible.panel = show,
            Zone::Agents => self.visible.agents = show,
        }
        // Zellij inserta el panel mostrado donde encuentra sitio: se recoloca
        // todo con el layout del estado actual. retain=true: nunca cierra paneles.
        override_layout(LayoutInfo::Stringified(layout::kdl(self.visible)), true, true, true, BTreeMap::new());
    }

    /// ⌘1–⌘5: foco por identidad; si el panel está oculto, primero se muestra.
    fn focus(&mut self, target: &str) {
        let (zone, pane) = match target {
            "explorer" => (Some(Zone::Sidebar), Some(PaneId::Plugin(self.me))),
            "editor" => (None, self.terminal("Editor")),
            "claude" => (Some(Zone::Agents), self.terminal("Claude Code")),
            "codex" => (Some(Zone::Agents), self.terminal("Codex")),
            "shell" => (Some(Zone::Panel), self.terminal("Shell")),
            _ => return,
        };
        if let Some(zone) = zone.filter(|z| !self.is_visible(*z)) {
            self.set_zone(zone, true, false);
        }
        if let Some(pane) = pane {
            focus_pane_with_id(pane, false, false);
        }
    }

    fn send_to_editor(&mut self, bytes: &[u8], focus: bool) {
        if let Some(editor) = self.terminal("Editor") {
            write_to_pane_id(bytes.to_vec(), editor);
            if focus {
                focus_pane_with_id(editor, false, false);
            }
        }
    }

    /// ⌘W: cierra la pestaña de Micro (Ctrl-Q) solo si el foco está en el editor.
    fn close_tab(&mut self) {
        if let Some(editor) = self.terminal("Editor") {
            if self.focused_in(&[editor]) {
                write_to_pane_id(vec![0x11], editor);
            }
        }
    }

    /// ⌘Z ⌘⇧Z ⌘A ⌘F ⌘X ⌘C, como VS Code: la tecla de Micro, solo con el foco
    /// en el editor. En un shell, Ctrl-Z suspendería el proceso y Ctrl-C lo
    /// interrumpiría. Ctrl-X y Ctrl-C son Cut|CutLine y Copy|CopyLine en
    /// Micro: sin selección, la línea entera.
    fn edit(&mut self, action: &str) {
        let key = match action {
            "undo" => 0x1a,
            "redo" => 0x19,
            "select_all" => 0x01,
            "find" => 0x06,
            "cut" => 0x18,
            "copy" => 0x03,
            _ => return,
        };
        if let Some(editor) = self.terminal("Editor") {
            if self.focused_in(&[editor]) {
                write_to_pane_id(vec![key], editor);
            }
        }
    }

    /// ⌘V: Ghostty ya ha pegado el texto. Si el foco está en Claude Code o
    /// Codex, se mira qué hay en el portapapeles: si es solo una imagen, el
    /// agente recibe ⌃V, con el que la adjunta (src/clipboard.rs).
    fn paste_image(&mut self) {
        let agent = ["Claude Code", "Codex"]
            .iter()
            .filter_map(|title| self.panes.iter().find(|p| !p.is_plugin && p.title == *title))
            .find(|p| self.has_focus(p));
        if let Some(agent) = agent {
            let context = BTreeMap::from([
                ("vkterm".to_string(), "paste".to_string()),
                ("pane".to_string(), agent.id.to_string()),
            ]);
            run_command(&["osascript", "-e", "clipboard info"], context);
        }
    }

    /// Desde la paleta: la ventana de atajos o el selector de tema, otra
    /// instancia flotante del plugin, como hacen los atajos de config.kdl.
    fn open_window(&mut self, kind: &str) {
        let (name, mode) = match kind {
            "shortcuts" => ("toggle_shortcuts", "shortcuts"),
            "themes" => ("toggle_themes", "themes"),
            _ => return,
        };
        pipe_message_to_plugin(
            MessageToPlugin::new(name)
                .with_plugin_url("vkterm")
                .with_plugin_config(BTreeMap::from([("mode".to_string(), mode.to_string())]))
                .new_plugin_instance_should_float(true),
        );
    }

    /// Colores del atenuado para el tema (o los de config.kdl, si los define).
    fn set_theme(&mut self, id: &str) {
        let theme = themes::find(id);
        let colors = self.colors_override.or_else(|| rgb(theme.inactive_fg).zip(rgb(theme.inactive_bg)));
        self.inactive_sgr = match colors {
            Some((fg, bg)) => format!("\u{1b}[38;2;{}m\u{1b}[48;2;{}m", sgr(fg), sgr(bg)),
            None => "\u{1b}[2m".to_string(),
        };
        self.inactive = colors.map(|(fg, bg)| (hex(fg), hex(bg)));
        self.dim_inactive();
    }

    /// Terminales sin foco con colores apagados, como iTerm2. Zellij solo
    /// cambia el texto y el fondo sin color propio: el resaltado de sintaxis,
    /// los prompts y las barras de Micro conservan sus colores.
    ///
    /// Se compara con los colores que Zellij informa de cada panel, no con lo
    /// último enviado: Zellij los reinicia al arrancar o terminar el comando
    /// de un panel y al redimensionar uno en espera ("Waiting to run"). Solo
    /// se quitan los colores puestos aquí, no los que fije un programa (OSC 10/11).
    fn dim_inactive(&self) {
        let Some((fg, bg)) = &self.inactive else {
            return;
        };
        for pane in self.panes.iter().filter(|p| !p.is_plugin) {
            let ours = pane.default_fg.as_ref() == Some(fg) && pane.default_bg.as_ref() == Some(bg);
            let focused = self.has_focus(pane);
            if focused && ours {
                set_pane_color(PaneId::Terminal(pane.id), None, None);
            } else if !focused && !ours {
                set_pane_color(PaneId::Terminal(pane.id), Some(fg.clone()), Some(bg.clone()));
            }
        }
    }

    /// Al cerrar la última pestaña Micro termina; como en VS Code, el editor
    /// vuelve a quedar disponible (vacío) en vez de un panel "EXITED".
    fn restart_editor_if_closed(&mut self) {
        let Some(editor) = self.panes.iter().find(|p| !p.is_plugin && p.title == "Editor") else {
            return;
        };
        if editor.exited && editor.exit_status == Some(0) {
            if !self.editor_restarting {
                self.editor_restarting = true;
                rerun_command_pane(editor.id);
            }
        } else {
            self.editor_restarting = false;
        }
    }
}

/// Recorta (con "…") o rellena con espacios hasta `width` columnas.
pub(crate) fn fit(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count > width {
        let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
        cut.push('…');
        cut
    } else {
        format!("{}{}", text, " ".repeat(width - count))
    }
}

/// "#rrggbb" (mayúsculas o minúsculas) → (r, g, b).
fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.strip_prefix('#').filter(|h| h.len() == 6)?;
    let n = u32::from_str_radix(hex, 16).ok()?;
    Some(((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// (r, g, b) → "#rrggbb", el formato con el que Zellij informa los colores.
fn hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// (r, g, b) → "r;g;b", para las secuencias SGR de 24 bits.
fn sgr((r, g, b): (u8, u8, u8)) -> String {
    format!("{r};{g};{b}")
}

/// Como Git: es binario si hay un byte NUL en los primeros 8 KB.
fn is_binary(path: &Path) -> bool {
    let mut head = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(8192).read_to_end(&mut head))
        .map(|_| head.contains(&0))
        .unwrap_or(false)
}
