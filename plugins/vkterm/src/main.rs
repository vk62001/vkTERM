//! vkterm: explorador tipo VS Code y controlador del workspace de vkTERM.
//!
//! - **Explorador**: muestra el proyecto como árbol en la barra lateral, con
//!   los colores de git de VS Code. Abre los archivos como pestañas del Micro
//!   del panel Editor (bin/vkterm-open). Nunca abre terminales.
//! - **Control de código fuente** (⌃⇧G): la otra vista de la barra lateral,
//!   con los cambios de git y los conflictos con otra rama (src/scm.rs). ⌘⇧E
//!   vuelve al explorador.
//! - **Controlador**: recibe los atajos (`MessagePlugin "vkterm"` en
//!   config.kdl). Oculta y muestra la barra lateral, el panel (Shell) y los
//!   agentes, y enfoca paneles por identidad. Sigue recibiendo atajos aunque
//!   su panel esté oculto.
//! - **Foco**: como "Dim inactive split panes" de iTerm2, los paneles sin foco
//!   se ven atenuados y el enfocado con sus colores normales.
//!
//! - **Memoria**: al abrir la carpeta deja el workspace como estaba (zonas,
//!   foco, carpetas desplegadas) y lo guarda cuando cambia (src/memory.rs,
//!   bin/vkterm-state). La sesión termina al cerrar la carpeta (⌘K F) o la
//!   pestaña de vkTERM.app.
//!
//! - **Temas y fuentes**: aplica el tema elegido en el selector (⌘K ⌘T) y la
//!   fuente (⌘K ⌘L, ⌘+, ⌘- y ⌘0), de uno en uno (src/applier.rs), y pinta la
//!   interfaz con los colores del tema de VS Code (src/style.rs).
//!
//! Con `mode "shortcuts"` en la configuración es, en cambio, la ventana de
//! atajos (src/overlay.rs); con `mode "themes"`, el selector de tema
//! (src/picker.rs); con `mode "fonts"`, el de fuentes (src/fonts.rs); con
//! `mode "palette"`, la paleta de archivos y comandos (src/palette.rs); con
//! `mode "recent"`, «Abrir reciente» (src/recent.rs); con
//! `mode "confirm"`, las confirmaciones (src/confirm.rs); con
//! `mode "branches"`, la rama con la que comparar (src/branches.rs);
//! con `mode "menu"`, el menú contextual del editor (src/contextmenu.rs); con
//! `mode "newfile"`, la ventana que pide el nombre de un archivo nuevo
//! (src/prompt.rs); con `mode "status"`, la barra de estado (src/status.rs).
//! Con otro `mode`, se cierra.
mod branches;
mod confirm;
mod contextmenu;
mod find;
mod floating;
mod fonts;
mod overlay;
mod palette;
mod picker;
mod prompt;
mod recent;
mod scm;
mod status;

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use branches::Branches;
use confirm::Confirm;
use contextmenu::ContextMenu;
use fonts::Fonts;
use overlay::Overlay;
use picker::Picker;
use prompt::Prompt;
use recent::Recent;
use find::Find;
use scm::Scm;
use status::StatusBar;
use vkterm::applier::{Applier, Steps};
use vkterm::clipboard;
use vkterm::closing::Loss;
use vkterm::commands;
use vkterm::layout::{self, Visible};
use vkterm::memory::Memory;
use vkterm::menu::{self, Menu};
use vkterm::newfile;
use vkterm::style::{Line, Look, Palette, Rgb};
use vkterm::themes;
use vkterm::tree::{Action, DirEntry, Tree};
use zellij_tile::prelude::*;

/// Carpeta del proyecto vista desde el plugin.
pub(crate) const HOST: &str = "/host";
const HEADER_ROWS: usize = 1;
const FOOTER_ROWS: usize = 1;
const HINT: &str = "Enter abrir · Espacio vista previa · ⌘B ocultar";
const EXPLORER_TITLE: &str = "Explorador";
/// Cada cuántos segundos se guarda la memoria si cambió (y se lee el estado
/// de git, si algo cambió en el disco).
const TICK: f64 = 1.0;
/// Cada cuántos ticks se vuelve a leer el estado de git aunque no cambie
/// nada en la carpeta (un commit desde fuera de ella).
const GIT_TICKS: u32 = 5;
const AGENTS: [&str; 2] = ["Claude Code", "Codex"];
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

/// La vista de la barra lateral, como en VS Code.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum View {
    #[default]
    Explorer,
    Scm,
    Search,
}

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
    /// tema, salvo que config.kdl defina inactive_fg e inactive_bg. None si
    /// el tema no atenúa (alto contraste).
    inactive: Option<(String, String)>,
    /// Los colores de atenuado puestos en esta sesión (de cualquier tema):
    /// solo se quitan estos, no los que fije un programa (OSC 10/11).
    dim_history: Vec<(String, String)>,
    colors_override: Option<(Rgb, Rgb)>,
    /// Colores de la interfaz: los del tema de Zellij (src/style.rs).
    palette: Palette,
    /// Aplicar el tema, la fuente y el tamaño de letra, de uno en uno.
    theme_applier: Applier,
    font_applier: Applier,
    size_steps: Steps,
    /// Ya se preguntó a bin/vkterm-theme por el tema guardado.
    asked_theme: bool,
    /// Esta instancia es una ventana flotante en lugar del explorador.
    window: Option<Window>,
    /// Lo elegido en la paleta, pendiente de que su ventana se cierre y el
    /// foco vuelva a los paneles fijos: (mensaje, payload).
    pending: Option<(String, String)>,
    /// El panel del menú contextual abierto: no se atenúa mientras se ve.
    menu_pane: Option<u32>,
    /// Memoria de la carpeta leída al arrancar, pendiente de aplicar.
    memory: Option<Memory>,
    /// Ya se aplicó la memoria: a partir de aquí se guarda.
    restored: bool,
    /// Lo último guardado (o leído), para guardar solo cuando cambia.
    saved_memory: String,
    /// Último panel fijo con el foco (con una ventana flotante a la vista, no
    /// hay ninguno).
    last_focus: Option<String>,
    ticks: u32,
    /// ⌘K F en curso: agentes en marcha mientras se pregunta a Micro.
    closing: Option<Vec<String>>,
    /// Carpeta que abrir en esta pestaña después de cerrar la actual («Abrir
    /// reciente» con Enter).
    next_folder: Option<String>,
    /// Barra de estado: instancias (una por pestaña de Zellij), rama, modo
    /// y lo último enviado.
    status_bars: Vec<u32>,
    mode_name: String,
    sent_status: String,
    /// La vista de la barra lateral y la de Control de código fuente.
    view: View,
    scm: Scm,
    find: Find,
}

enum Window {
    Shortcuts(Overlay),
    Themes(Picker),
    Fonts(Fonts),
    Palette(palette::Palette),
    Recent(Recent),
    Confirm(Confirm),
    Menu(ContextMenu),
    NewFile(Prompt),
    Branches(Branches),
    Status(StatusBar),
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
            Some("fonts") => {
                let mut fonts = Fonts::default();
                fonts.load();
                self.window = Some(Window::Fonts(fonts));
                return;
            }
            Some("palette") => {
                let mut palette = palette::Palette::default();
                palette.load();
                self.window = Some(Window::Palette(palette));
                return;
            }
            Some("recent") => {
                let mut recent = Recent::default();
                recent.load();
                self.window = Some(Window::Recent(recent));
                return;
            }
            Some("confirm") => {
                let mut confirm = Confirm::default();
                confirm.load();
                self.window = Some(Window::Confirm(confirm));
                return;
            }
            Some("menu") => {
                let mut menu = ContextMenu::default();
                menu.load();
                self.window = Some(Window::Menu(menu));
                return;
            }
            Some("newfile") => {
                let mut prompt = Prompt::default();
                prompt.load();
                self.window = Some(Window::NewFile(prompt));
                return;
            }
            Some("branches") => {
                let mut branches = Branches::default();
                branches.load();
                self.window = Some(Window::Branches(branches));
                return;
            }
            Some("status") => {
                let mut bar = StatusBar::default();
                bar.load();
                self.window = Some(Window::Status(bar));
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
        let color = |key: &str| configuration.get(key).and_then(|value| Rgb::parse(value));
        self.colors_override = color("inactive_fg").zip(color("inactive_bg"));
        self.theme_colors(themes::DEFAULT);
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
            EventType::CommandPaneExited,
            EventType::Timer,
        ]);
        set_selectable(true);
        set_timeout(TICK);
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
            Some(Window::Fonts(fonts)) => return fonts.update(event),
            Some(Window::Palette(palette)) => return palette.update(event),
            Some(Window::Recent(recent)) => return recent.update(event),
            Some(Window::Confirm(confirm)) => return confirm.update(event),
            Some(Window::Menu(menu)) => return menu.update(event),
            Some(Window::NewFile(prompt)) => return prompt.update(event),
            Some(Window::Branches(branches)) => return branches.update(event),
            Some(Window::Status(_)) => return false,
            Some(Window::Closed) => return false,
            None => {}
        }
        match event {
            Event::PaneUpdate(manifest) => {
                let bars = self.status_bar_ids(&manifest);
                for (tab, panes) in manifest.panes {
                    if panes.iter().any(|p| p.is_plugin && p.id == self.me) {
                        self.my_tab = tab;
                        self.panes = panes;
                    }
                }
                self.set_status_bars(bars);
                self.restart_editor_if_closed();
                self.dim_inactive();
                self.try_restore();
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
                if !floating {
                    self.menu_pane = None;
                }
                self.dim_inactive();
                true
            }
            Event::ModeUpdate(mode) => {
                self.session = mode.session_name;
                // Llega ya con los permisos concedidos.
                if !self.asked_theme {
                    self.asked_theme = true;
                    run_command(&["vkterm-theme", "current"], context("current"));
                    run_command(&["vkterm-state", "load"], context("memory"));
                    self.scm.refresh(&self.project);
                }
                self.mode_name = match mode.mode {
                    InputMode::Locked => String::new(),
                    other => format!("{other:?}").to_uppercase(),
                };
                // También llega al cambiar el tema de Zellij: la barra de estado
                // no recibe eventos y se vuelve a dibujar con cada mensaje.
                let palette = floating::palette(&mode.style);
                let changed = palette != self.palette;
                self.palette = palette;
                self.send_status(true);
                changed
            }
            Event::Timer(_) => {
                self.tick();
                set_timeout(TICK);
                false
            }
            Event::RunCommandResult(code, stdout, stderr, ctx) if ctx.contains_key("vkterm") => {
                let stdout = String::from_utf8_lossy(&stdout);
                match ctx["vkterm"].as_str() {
                    "git_status" => {
                        self.scm.status_result(code, &stdout, &self.project);
                        self.send_status(false);
                        true
                    }
                    kind @ ("git_action" | "git_commit") => {
                        let stderr = String::from_utf8_lossy(&stderr);
                        self.scm.action_result(kind, code, &stdout, &stderr, &self.project);
                        true
                    }
                    "search" => {
                        let stderr = String::from_utf8_lossy(&stderr);
                        self.find.result(code, &stdout, &stderr, &self.project);
                        true
                    }
                    "replace" => {
                        let stderr = String::from_utf8_lossy(&stderr);
                        self.find.replace_result(code, &stdout, &stderr, &self.project);
                        true
                    }
                    _ => self.command_result(&ctx, &stdout),
                }
            }
            Event::CommandPaneExited(id, code, ctx) if ctx.contains_key("vkterm") => {
                self.scm.window_exited(id, code, &self.project)
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
            Some(Window::Fonts(fonts)) => return fonts.pipe(message),
            Some(Window::Palette(palette)) => return palette.pipe(message),
            Some(Window::Recent(recent)) => return recent.pipe(message),
            Some(Window::Confirm(confirm)) => return confirm.pipe(message),
            Some(Window::Menu(menu)) => return menu.pipe(message),
            Some(Window::NewFile(prompt)) => return prompt.pipe(message),
            Some(Window::Branches(branches)) => return branches.pipe(message),
            Some(Window::Status(bar)) => return bar.pipe(message),
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
            Some(Window::Fonts(fonts)) => return fonts.render(rows, cols),
            Some(Window::Palette(palette)) => return palette.render(rows, cols),
            Some(Window::Recent(recent)) => return recent.render(rows, cols),
            Some(Window::Confirm(confirm)) => return confirm.render(rows, cols),
            Some(Window::Menu(menu)) => return menu.render(rows, cols),
            Some(Window::NewFile(prompt)) => return prompt.render(rows, cols),
            Some(Window::Branches(branches)) => return branches.render(rows, cols),
            Some(Window::Status(bar)) => return bar.render(rows, cols),
            Some(Window::Closed) => return,
            None => {}
        }
        self.rows = rows;
        // Las líneas con fondo de color no se recortan al estrechar el panel
        // (al volver a mostrarlo con ⌘B): Zellij las parte y las pasa al
        // scroll. Cada render vacía el panel, pero el marco seguiría mostrando
        // "SCROLL: 0/N"; CSI 3 J pone el contador a cero.
        print!("\u{1b}[3J");
        // Con los colores de la barra lateral del tema de VS Code; sin foco,
        // los de los terminales sin foco (dim_inactive), si el tema atenúa.
        let focused = self.sidebar_focused();
        let p = self.palette;
        let dim = if focused { None } else { self.inactive_look() };
        if self.view == View::Scm {
            self.scm.render(rows, cols, focused, &p, dim);
            return;
        }
        if self.view == View::Search {
            self.find.render(rows, cols, focused, &p, dim);
            return;
        }
        let base = dim.unwrap_or(p.sidebar);
        let name = self
            .project
            .file_name()
            .map(|n| n.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        print!("{}", Line::new(&format!(" {}", name), cols, dim.unwrap_or(p.header).bold()).render(0, 0));

        let list = self.tree.rows();
        let selected = self.tree.selected_index(&list);
        let git = self.scm.colors;
        // Como VS Code: el nombre en el color de su estado de git y la letra a
        // la derecha (en una carpeta, un punto); lo ignorado, apagado.
        let label_width = cols.saturating_sub(3);
        for y in 0..self.list_height() {
            let i = self.tree.scroll + y;
            let Some(row) = list.get(i) else {
                print!("{}", Line::new("", cols, base).render(0, HEADER_ROWS + y));
                continue;
            };
            let icon = match (row.is_dir, row.expanded) {
                (true, true) => "▾ ",
                (true, false) => "▸ ",
                _ => "  ",
            };
            let text = format!(" {}{}{}", "  ".repeat(row.depth), icon, row.name);
            let kind = self.scm.decorations.get(&row.path, row.is_dir);
            let letter = match kind {
                Some(_) if row.is_dir => '•',
                Some(kind) => kind.letter(),
                None => ' ',
            };
            let mut line = Line::new(&format!("{} {letter} ", fit(&text, label_width)), cols, base);
            if i == selected {
                let look = if focused { p.selection } else { p.inactive_selection };
                line.paint(0..cols, |_| look);
            }
            let color = match kind {
                Some(kind) => Some(scm::kind_color(&git, kind)),
                None if self.scm.decorations.ignored(&row.path) => Some(git.ignored),
                None => None,
            };
            if let Some(color) = color {
                let start = 1 + 2 * row.depth + 2;
                line.fg(start..(start + row.name.chars().count()).min(label_width), color);
                line.fg(cols.saturating_sub(2)..cols.saturating_sub(1), color);
            }
            print!("{}", line.render(0, HEADER_ROWS + y));
        }

        let mut footer = Line::new(&format!(" {}", self.status.as_deref().unwrap_or(HINT)), cols, base);
        footer.fg(0..cols, if self.status.is_some() { p.error } else { dim.map_or(p.description, |d| d.fg) });
        print!("{}", footer.render(0, rows.saturating_sub(1)));
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
            // Clic derecho en Micro (config/micro/plugins/vkterm) y lo elegido
            // en su menú (src/contextmenu.rs).
            "context_menu" => self.context_menu(payload.trim()),
            "menu_action" => self.menu_action(payload.trim()),
            // De la paleta (src/commands.rs).
            "open" => {
                self.status = None;
                self.open(Path::new(payload.trim()), true);
            }
            // ⌘N: pide el nombre (src/prompt.rs) y lo crea.
            "new_file" => self.new_file(),
            // Clic derecho en el explorador: la entrada en Finder.
            "reveal_file" => self.reveal_file(),
            "create_file" => self.create_file(Path::new(payload.trim())),
            // ⌘R: pide el nombre nuevo (src/prompt.rs) y renombra.
            "rename_file" => self.rename_file(),
            "rename_to" => {
                if let Some((old, new)) = payload.split_once('\t') {
                    self.rename_to(Path::new(old), Path::new(new.trim()));
                }
            }
            "editor_key" => {
                if let Some(bytes) = commands::editor_key(payload.trim()) {
                    self.send_to_editor(bytes, true);
                }
            }
            "zoom" => toggle_focus_fullscreen(),
            "toggle_excluded" => self.tree.show_excluded = !self.tree.show_excluded,
            // ⌘⇧E y ⌃⇧G: la vista de la barra lateral, como en VS Code.
            "show_view" => self.show_view(payload.trim()),
            // ⌘Enter: commit, solo en la vista de Control de código fuente.
            "scm_commit" => {
                if self.view == View::Scm && self.sidebar_focused() {
                    self.scm.commit();
                }
            }
            // Comandos «Git: …» de la paleta.
            "git" => {
                if payload.trim() == "commit" {
                    self.show_view("scm");
                }
                self.scm.command(payload.trim(), &self.project);
            }
            // Las respuestas de la confirmación (src/confirm.rs) y de la
            // rama con la que comparar (src/branches.rs).
            "scm_answer" => self.scm.answer(),
            // La confirmación de «Reemplazar en los archivos» (src/find.rs).
            "search_answer" => self.find.answer(&self.project),
            "compare_branch" => {
                self.scm.target = payload.trim().to_string();
                self.scm.refresh(&self.project);
            }
            "open_window" => self.open_window(payload.trim()),
            // ⌘K F y la respuesta de su confirmación (src/confirm.rs).
            "close_folder" => self.close_folder(None),
            "close_folder_confirmed" => self.close_confirmed(payload.trim()),
            // Lo elegido en «Abrir reciente» (src/recent.rs).
            "open_recent" => self.open_recent(payload.trim_end_matches('\n')),
            // ⌘⇧,: Ghostty relee su configuración y la pestaña sigue en modo IDE.
            "reload_config" => self.run_with_session(&["vkterm-theme", "reload"], "reload"),
            // Del selector de tema (src/picker.rs) y del de fuentes (src/fonts.rs).
            "set_theme" => self.preview_theme(payload.trim()),
            "set_font" => self.set_font(payload.trim()),
            // ⌘+, ⌘- y ⌘0 (y ← → en el selector de fuentes): "+1", "-1" o "0".
            "font_size" => self.font_size(payload.trim()),
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
        if paths.iter().any(|p| self.git_relevant(p)) {
            self.scm.mark_dirty();
        }
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

    /// Si un cambio en el disco puede cambiar el estado de git: no los de
    /// lo ignorado (target/ al compilar) ni los objetos de .git.
    fn git_relevant(&self, path: &Path) -> bool {
        let Some(relative) = self.relative(path) else { return true };
        let mut parts = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned());
        match parts.next().as_deref() {
            Some(".git") => matches!(
                parts.next().as_deref(),
                Some(
                    "HEAD" | "index" | "refs" | "packed-refs" | "MERGE_HEAD" | "ORIG_HEAD" | "CHERRY_PICK_HEAD"
                        | "REVERT_HEAD" | "rebase-merge" | "rebase-apply"
                )
            ),
            _ => !self.scm.decorations.ignored(&relative),
        }
    }

    fn relative(&self, path: &Path) -> Option<PathBuf> {
        path.strip_prefix(HOST)
            .or_else(|_| path.strip_prefix(&self.project))
            .ok()
            .map(Path::to_path_buf)
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        if self.view == View::Scm {
            let request = self.scm.key(key);
            return self.scm_request(request);
        }
        if self.view == View::Search {
            let request = self.find.key(key, &self.project);
            return self.find_request(request);
        }
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
        if self.view == View::Scm {
            let request = self.scm.mouse(mouse);
            return self.scm_request(request);
        }
        if self.view == View::Search {
            let request = self.find.mouse(mouse);
            return self.find_request(request);
        }
        match mouse {
            Mouse::LeftClick(line, _) if line >= HEADER_ROWS as isize => {
                let index = line as usize - HEADER_ROWS + self.tree.scroll;
                let action = self.tree.click(index);
                self.status = None;
                self.run(action, true);
            }
            // Como en VS Code, selecciona la entrada y abre su menú junto al puntero.
            Mouse::RightClick(line, column) if line >= HEADER_ROWS as isize => {
                let index = line as usize - HEADER_ROWS + self.tree.scroll;
                self.tree.select_at(index);
                self.status = None;
                let me = self.panes.iter().find(|p| p.is_plugin && p.id == self.me);
                let click = me.map_or((column, line as usize), |p| {
                    (p.pane_content_x + column, p.pane_content_y + line as usize)
                });
                self.open_menu("explorer", menu::EXPLORER, click);
            }
            Mouse::ScrollUp(n) => self.tree.scroll_by(-(n as isize), self.list_height()),
            Mouse::ScrollDown(n) => self.tree.scroll_by(n as isize, self.list_height()),
            _ => return false,
        }
        true
    }

    fn find_request(&mut self, request: find::Request) -> bool {
        match request {
            find::Request::Ignored => return false,
            find::Request::Redraw => {}
            find::Request::Open(file, line, column) => self.open_at(&file, line, column),
        }
        true
    }

    fn scm_request(&mut self, request: scm::Request) -> bool {
        match request {
            scm::Request::Ignored => return false,
            scm::Request::Redraw => {}
            // Relativa al proyecto o, si está fuera de él, absoluta.
            scm::Request::Open(file) => self.open(&file, true),
        }
        true
    }

    fn sidebar_focused(&self) -> bool {
        self.panes.iter().any(|p| p.is_plugin && p.id == self.me && self.has_focus(p))
    }

    /// ⌘⇧E (explorer) y ⌃⇧G (scm): muestra la barra lateral con esa vista y
    /// le da el foco; en Control de código fuente, al mensaje del commit.
    fn show_view(&mut self, view: &str) {
        let view = match view {
            "scm" => View::Scm,
            "search" => View::Search,
            _ => View::Explorer,
        };
        if view == View::Search {
            self.find.focus_query();
        }
        if view == View::Scm {
            self.scm.focus_message();
            if self.view != View::Scm {
                self.scm.refresh(&self.project);
            }
        }
        self.view = view;
        self.status = None;
        rename_plugin_pane(self.me, self.view_title());
        self.focus("explorer");
    }

    fn view_title(&self) -> &'static str {
        match self.view {
            View::Explorer => EXPLORER_TITLE,
            View::Scm => scm::TITLE,
            View::Search => find::TITLE,
        }
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
        self.open_with(file, focus, 0, 0)
    }

    /// Lo mismo, llevando el cursor a `line`:`column` (0 = donde estaba).
    fn open_at(&mut self, file: &Path, line: usize, column: usize) {
        self.open_with(file, true, line, column)
    }

    fn open_with(&mut self, file: &Path, focus: bool, line: usize, column: usize) {
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
        if line > 0 {
            env.insert("VKTERM_LINE".to_string(), line.to_string());
            env.insert("VKTERM_COLUMN".to_string(), column.max(1).to_string());
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
        if self.change_zone(zone, show, focus) {
            self.apply_layout();
        }
    }

    /// Muestra u oculta la zona sin recolocar nada; true si cambió.
    fn change_zone(&mut self, zone: Zone, show: bool, focus: bool) -> bool {
        // Con otra pestaña de Zellij activa, el layout se aplicaría a esa.
        if self.active_tab != self.my_tab {
            return false;
        }
        let ids = self.zone_panes(zone);
        if ids.is_empty() {
            return false;
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
        true
    }

    /// Zellij inserta el panel mostrado donde encuentra sitio: se recoloca
    /// todo con el layout del estado actual. retain=true: nunca cierra paneles.
    fn apply_layout(&self) {
        override_layout(LayoutInfo::Stringified(layout::kdl(self.visible)), true, true, true, BTreeMap::new());
        // El layout lo llama «Explorador»: el título es el de la vista.
        if self.view != View::Explorer {
            rename_plugin_pane(self.me, self.view_title());
        }
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
            "undo" => vec![0x1a],
            "redo" => vec![0x19],
            "select_all" => vec![0x01],
            "find" => vec![0x06],
            "cut" => vec![0x18],
            "copy" => vec![0x03],
            // Ctrl-E y el comando del plugin Lua (config/micro/plugins/vkterm).
            "replace" => commands::editor_key("replace").unwrap_or_default().to_vec(),
            _ => return,
        };
        if let Some(editor) = self.terminal("Editor") {
            if self.focused_in(&[editor]) {
                write_to_pane_id(key, editor);
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

    /// Clic derecho en un Micro de la sesión: "panel columna fila", la
    /// posición dentro del panel. Como en VS Code, el panel recibe el foco y
    /// el menú contextual (src/contextmenu.rs) se abre ya colocado en el
    /// puntero, dentro de la zona de paneles.
    fn context_menu(&mut self, payload: &str) {
        let numbers: Vec<usize> = payload.split_whitespace().filter_map(|n| n.parse().ok()).collect();
        let [id, x, y] = numbers[..] else { return };
        let Some(pane) = self.panes.iter().find(|p| !p.is_plugin && !p.is_suppressed && p.id as usize == id) else {
            return;
        };
        let click = (pane.pane_content_x + x, pane.pane_content_y + y);
        let pane = pane.id;
        // El botón se suelta ya con el menú abierto y Zellij manda el «soltar»
        // al panel con el foco, el menú: sin él, Micro no haría caso del
        // siguiente clic derecho.
        write_to_pane_id(format!("\u{1b}[<2;{};{}m", x + 1, y + 1).into_bytes(), PaneId::Terminal(pane));
        focus_pane_with_id(PaneId::Terminal(pane), false, false);
        self.menu_pane = Some(pane);
        self.open_menu(&pane.to_string(), menu::EDITOR, click);
    }

    /// Abre el menú contextual (src/contextmenu.rs) junto al puntero, dentro de
    /// la zona de paneles. `target` es el panel de Micro o "explorer"; `click`,
    /// la celda del clic en la ventana.
    fn open_menu(&mut self, target: &str, entries: &'static [menu::Entry], click: (usize, usize)) {
        let tiled = self.panes.iter().filter(|p| !p.is_floating && !p.is_suppressed && p.is_selectable);
        let area = (
            tiled.clone().map(|p| p.pane_x + p.pane_columns).max().unwrap_or(0),
            tiled.map(|p| p.pane_y + p.pane_rows).max().unwrap_or(0),
        );
        let size = Menu::new(entries).size();
        let (left, top) = menu::position(click, size, area);
        let coordinates = FloatingPaneCoordinates::new(
            Some(left.to_string()),
            Some(top.to_string()),
            Some(size.0.to_string()),
            Some(size.1.to_string()),
            None,
            Some(true),
        );
        let mut message = MessageToPlugin::new("menu")
            .with_plugin_url("vkterm")
            .with_plugin_config(BTreeMap::from([("mode".to_string(), "menu".to_string())]))
            .new_plugin_instance_should_float(true)
            .with_payload(format!("{target} {left} {top}"));
        if let Some(coordinates) = coordinates {
            message = message.with_floating_pane_coordinates(coordinates);
        }
        pipe_message_to_plugin(message);
    }

    /// Lo elegido en el menú contextual: "panel acción". La tecla de Micro
    /// (src/commands.rs) en el panel del clic, o la paleta de comandos.
    fn menu_action(&mut self, payload: &str) {
        let Some((pane, action)) = payload.split_once(' ') else { return };
        if pane == "explorer" {
            if matches!(action, "new_file" | "rename_file" | "toggle_excluded" | "reveal_file") {
                self.dispatch(action, "");
            } else if action == "command_palette" {
                self.open_window("palette_commands");
            }
            return;
        }
        let Some(pane) = pane.parse().ok().filter(|id| self.panes.iter().any(|p| !p.is_plugin && p.id == *id)) else {
            return;
        };
        if action == "command_palette" {
            self.open_window("palette_commands");
        } else if let Some(bytes) = commands::editor_key(action) {
            write_to_pane_id(bytes.to_vec(), PaneId::Terminal(pane));
            focus_pane_with_id(PaneId::Terminal(pane), false, false);
        }
    }

    /// ⌘N «Nuevo archivo…», como en el explorador de VS Code: en la carpeta
    /// seleccionada (o en la del archivo seleccionado).
    fn new_file(&mut self) {
        let row = self.tree.selected_path().and(self.tree.selected());
        let folder = newfile::folder(row.as_ref().map(|r| (r.path.as_path(), r.is_dir)));
        launch_window("new_file", "newfile", Some(folder.to_string_lossy().into_owned()));
    }

    /// «Mostrar en Finder» la entrada seleccionada del explorador (`open -R`).
    fn reveal_file(&mut self) {
        let Some(path) = self.tree.selected_path() else {
            self.status = Some("Selecciona en el explorador lo que quieras mostrar".to_string());
            return;
        };
        let path = if path.is_absolute() { path } else { self.project.join(path) };
        run_command(&["open", "-R", &path.to_string_lossy()], context("reveal"));
    }

    /// ⌘R «Renombrar…» el archivo o carpeta seleccionado del explorador.
    fn rename_file(&mut self) {
        match self.tree.selected_path().and(self.tree.selected()) {
            Some(row) => launch_window("rename_file", "newfile", Some(row.path.to_string_lossy().into_owned())),
            None => self.status = Some("Selecciona en el explorador lo que quieras renombrar".to_string()),
        }
    }

    /// De la ventana «Renombrar»: mueve en el disco, actualiza el explorador y
    /// abre el archivo con su nombre nuevo. Una pestaña del editor con el
    /// nombre viejo sigue abierta: hay que cerrarla (guardarla recrearía el
    /// archivo viejo).
    fn rename_to(&mut self, old: &Path, new: &Path) {
        let (host_old, host_new) = (Path::new(HOST).join(old), Path::new(HOST).join(new));
        let moved = host_new
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::rename(&host_old, &host_new));
        if let Err(error) = moved {
            self.status = Some(format!("{}: {error}", old.display()));
            return;
        }
        let was_dir = host_new.is_dir();
        // Se vuelven a leer las carpetas de las dos puntas y las desplegadas
        // bajo la vieja (si era una carpeta) se olvidan.
        let mut stale: Vec<PathBuf> = self.tree.loaded_dirs().into_iter().filter(|d| d.starts_with(old)).collect();
        stale.extend(old.parent().map(Path::to_path_buf));
        for dir in stale {
            if dir.starts_with(old) {
                self.tree.forget(&dir);
            } else {
                self.load_dir(&dir);
            }
        }
        for dir in self.tree.reveal(new) {
            self.load_dir(&dir);
        }
        self.tree.keep_visible(self.list_height());
        self.status = None;
        if !was_dir {
            self.open(new, true);
        }
    }

    /// De la ventana «Nuevo archivo»: lo crea vacío (y sus carpetas), lo
    /// selecciona en el explorador y lo abre en una pestaña del editor.
    fn create_file(&mut self, file: &Path) {
        let host = Path::new(HOST).join(file);
        let created = host
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::OpenOptions::new().write(true).create_new(true).open(&host));
        if let Err(error) = created {
            self.status = Some(format!("{}: {error}", file.display()));
            return;
        }
        for dir in self.tree.reveal(file) {
            self.load_dir(&dir);
        }
        self.tree.keep_visible(self.list_height());
        self.status = None;
        self.open(file, true);
    }

    /// Desde la paleta o al abrir la carpeta (`vkterm --then`): la ventana de
    /// atajos, el selector de tema o la paleta, otra instancia flotante del
    /// plugin, como hacen los atajos de config.kdl.
    fn open_window(&mut self, kind: &str) {
        let (name, mode) = match kind {
            "shortcuts" => ("toggle_shortcuts", "shortcuts"),
            "themes" => ("toggle_themes", "themes"),
            "fonts" => ("toggle_fonts", "fonts"),
            "palette_files" => ("palette_files", "palette"),
            "palette_commands" => ("palette_commands", "palette"),
            "recent" => ("toggle_recent", "recent"),
            _ => return,
        };
        launch_window(name, mode, None);
    }

    /// Del selector de tema: lo prueba (o lo deja). Zellij y los colores del
    /// atenuado cambian al momento; Ghostty y Micro, con bin/vkterm-theme, de
    /// uno en uno: cada vez recarga Ghostty (src/applier.rs).
    fn preview_theme(&mut self, id: &str) {
        let theme = themes::find(id);
        self.theme_colors(theme.id);
        reconfigure(format!("theme \"{}\"", theme.file()), false);
        if let Some(next) = self.theme_applier.want(theme.id) {
            self.run_with_session(&["vkterm-theme", "set", &next], "theme_set");
        }
    }

    /// Del selector de fuentes: la prueba (o la deja), de una en una.
    fn set_font(&mut self, family: &str) {
        if let Some(next) = self.font_applier.want(family) {
            self.run_with_session(&["vkterm-font", "set", &next], "font_set");
        }
    }

    fn font_size(&mut self, step: &str) {
        let next = match step {
            "0" => self.size_steps.reset(),
            step => step.parse().ok().and_then(|delta| self.size_steps.step(delta)),
        };
        if let Some(arg) = next {
            self.run_with_session(&["vkterm-font", "size", &arg], "font_size");
        }
    }

    /// Un comando de bin/ con la sesión de Zellij en el entorno: bin/vkterm-theme
    /// la usa para encontrar el Ghostty que hay que recargar.
    fn run_with_session(&self, args: &[&str], kind: &str) {
        let mut env = BTreeMap::new();
        if let Some(session) = &self.session {
            env.insert("ZELLIJ_SESSION_NAME".to_string(), session.clone());
        }
        run_command_with_env_variables_and_cwd(args, env, self.project.clone(), context(kind));
    }

    /// Colores del atenuado para el tema (o los de config.kdl, si los define).
    fn theme_colors(&mut self, id: &str) {
        let theme = themes::find(id);
        self.scm.colors = theme.git_colors();
        let colors = match (self.colors_override, theme.dim) {
            (Some(colors), _) => Some(colors),
            (None, true) => Rgb::parse(theme.inactive_fg).zip(Rgb::parse(theme.inactive_bg)),
            (None, false) => None,
        };
        self.inactive = colors.map(|(fg, bg)| (fg.hex(), bg.hex()));
        self.dim_inactive();
    }

    /// Colores del explorador sin foco, si el tema atenúa.
    fn inactive_look(&self) -> Option<Look> {
        let (fg, bg) = self.inactive.as_ref()?;
        Some(Look::new(Rgb::parse(fg)?, Rgb::parse(bg)?))
    }

    /// Terminales sin foco con colores apagados, como iTerm2. Zellij solo
    /// cambia el texto y el fondo sin color propio: el resaltado de sintaxis,
    /// los prompts y las barras de Micro conservan sus colores.
    ///
    /// Se compara con los colores que Zellij informa de cada panel, no con lo
    /// último enviado: Zellij los reinicia al arrancar o terminar el comando
    /// de un panel y al redimensionar uno en espera ("Waiting to run"). Solo
    /// se quitan los colores puestos aquí (de este tema o de uno anterior que
    /// Zellij aún no ha informado), no los que fije un programa (OSC 10/11).
    /// Con un tema que no atenúa (alto contraste), se quitan en todos.
    fn dim_inactive(&mut self) {
        if let Some(colors) = &self.inactive {
            if !self.dim_history.contains(colors) {
                self.dim_history.push(colors.clone());
            }
        }
        for pane in self.panes.iter().filter(|p| !p.is_plugin) {
            let current = pane.default_fg.clone().zip(pane.default_bg.clone());
            let ours = current.as_ref().is_some_and(|c| self.dim_history.contains(c));
            match &self.inactive {
                Some((fg, bg)) if !self.has_focus(pane) && self.menu_pane != Some(pane.id) => {
                    if current.as_ref() != Some(&(fg.clone(), bg.clone())) {
                        set_pane_color(PaneId::Terminal(pane.id), Some(fg.clone()), Some(bg.clone()));
                    }
                }
                _ if ours => set_pane_color(PaneId::Terminal(pane.id), None, None),
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------- memoria

    /// Resultados de `run_command` (contexto `vkterm`).
    fn command_result(&mut self, ctx: &BTreeMap<String, String>, stdout: &str) -> bool {
        match ctx["vkterm"].as_str() {
            "current" => self.theme_colors(stdout),
            "theme_set" => {
                if let Some(next) = self.theme_applier.finished() {
                    self.run_with_session(&["vkterm-theme", "set", &next], "theme_set");
                }
                return false;
            }
            "font_set" => {
                if let Some(next) = self.font_applier.finished() {
                    self.run_with_session(&["vkterm-font", "set", &next], "font_set");
                }
                return false;
            }
            "font_size" => {
                if let Some(arg) = self.size_steps.finished() {
                    self.run_with_session(&["vkterm-font", "size", &arg], "font_size");
                }
                return false;
            }
            "paste" => {
                let pane = ctx.get("pane").and_then(|id| id.parse().ok());
                if let Some(pane) = pane.filter(|_| clipboard::image_only(stdout)) {
                    write_to_pane_id(vec![0x16], PaneId::Terminal(pane));
                }
                return false;
            }
            "memory" => {
                let memory = Memory::parse(stdout);
                self.saved_memory = memory.to_text();
                self.memory = Some(memory);
                self.try_restore();
            }
            "close_check" => self.close_checked(stdout),
            "close_save" if stdout.trim() == "saved" => self.quit(),
            "close_save" => {
                self.closing = None;
                self.status = Some("No se pudieron guardar todos los archivos: la carpeta sigue abierta".to_string());
            }
            "close_discard" => self.quit(),
            "next" => self.quit(),
            "final" => quit_zellij(),
            _ => return false,
        }
        true
    }

    /// Aplica la memoria una sola vez, cuando ya están todos los paneles:
    /// carpetas desplegadas, zonas visibles, foco y la acción de `--then`.
    fn try_restore(&mut self) {
        if self.restored || self.active_tab != self.my_tab {
            return;
        }
        let Some(memory) = self.memory.take() else { return };
        let ready = self.panes.iter().any(|p| p.is_plugin && p.id == self.me)
            && ["Editor", "Shell", "Claude Code", "Codex"].iter().all(|t| self.terminal(t).is_some());
        if !ready {
            self.memory = Some(memory);
            return;
        }
        self.restored = true;
        self.tree.show_excluded = memory.show_excluded;
        if let Some(target) = &memory.compare {
            self.scm.target = target.clone();
            self.scm.refresh(&self.project);
        }
        if memory.view.as_deref() == Some("scm") {
            self.view = View::Scm;
            rename_plugin_pane(self.me, self.view_title());
        }
        for dir in self.tree.restore(&memory.expanded, memory.selected.clone()) {
            self.load_dir(&dir);
        }
        self.tree.keep_visible(self.list_height());
        let mut changed = false;
        for (zone, show) in [
            (Zone::Sidebar, memory.visible.sidebar),
            (Zone::Panel, memory.visible.panel),
            (Zone::Agents, memory.visible.agents),
        ] {
            if !show {
                changed |= self.change_zone(zone, false, false);
            }
        }
        if changed {
            self.apply_layout();
        }
        if let Some(target) = &memory.focus {
            self.focus(target);
        }
        self.last_focus = memory.focus.clone();
        match memory.then.as_deref() {
            Some("show_sidebar") => self.focus("explorer"),
            Some("show_panel") => self.focus("shell"),
            Some("quick_open") => self.open_window("palette_files"),
            Some("command_palette") => self.open_window("palette_commands"),
            _ => {}
        }
    }

    /// Panel fijo con el foco, por identidad (como `focus`).
    fn focused_target(&self) -> Option<&'static str> {
        if self.floating_active {
            return None;
        }
        let pane = self.panes.iter().find(|p| !p.is_floating && self.has_focus(p))?;
        if pane.is_plugin {
            return (pane.id == self.me).then_some("explorer");
        }
        match pane.title.as_str() {
            "Editor" => Some("editor"),
            "Shell" => Some("shell"),
            "Claude Code" => Some("claude"),
            "Codex" => Some("codex"),
            _ => None,
        }
    }

    fn snapshot(&mut self) -> String {
        if let Some(target) = self.focused_target() {
            self.last_focus = Some(target.to_string());
        }
        Memory {
            visible: self.visible,
            focus: self.last_focus.clone(),
            expanded: self.tree.expanded_dirs(),
            selected: self.tree.selected_path(),
            show_excluded: self.tree.show_excluded,
            view: (self.view == View::Scm).then(|| "scm".to_string()),
            compare: (!self.scm.target.is_empty()).then(|| self.scm.target.clone()),
            then: None,
        }
        .to_text()
    }

    /// Cada segundo: lee el estado de git si algo cambió (y, de vez en cuando,
    /// aunque no) y guarda la memoria si cambió (solo después de aplicarla,
    /// para no pisarla con el estado inicial).
    fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks % GIT_TICKS == 0 {
            self.scm.mark_dirty();
        }
        if self.asked_theme {
            self.scm.tick(&self.project);
            self.find.tick(&self.project);
        }
        if !self.restored || self.closing.is_some() {
            return;
        }
        let text = self.snapshot();
        if text != self.saved_memory {
            run_command(&["vkterm-state", "save", "workspace", &text], BTreeMap::new());
            self.saved_memory = text;
        }
    }

    // ------------------------------------------------------- cerrar carpeta

    /// ⌘K F: si no se pierde nada, cierra; si no, pregunta (src/confirm.rs).
    /// Lo modificado en el editor lo sabe el plugin Lua de Micro. Con `next`
    /// («Abrir reciente»), la pestaña abre después esa carpeta.
    fn close_folder(&mut self, next: Option<String>) {
        if self.closing.is_some() {
            return;
        }
        self.next_folder = next;
        let running: Vec<String> = AGENTS
            .iter()
            .filter(|t| self.panes.iter().any(|p| !p.is_plugin && p.title == **t && !p.exited && !p.is_held))
            .map(|t| t.to_string())
            .collect();
        self.closing = Some(running);
        run_command(&["vkterm-state", "close-check"], context("close_check"));
    }

    fn close_checked(&mut self, editor: &str) {
        let mut loss = Loss {
            project: self.project_name(),
            agents: self.closing.take().unwrap_or_default(),
            ..Default::default()
        };
        loss.read_editor(editor, &self.project);
        if loss.is_empty() {
            self.closing = Some(Vec::new());
            self.quit();
        } else {
            launch_window("close_folder", "confirm", Some(loss.to_text()));
        }
    }

    fn close_confirmed(&mut self, choice: &str) {
        self.closing = Some(Vec::new());
        match choice {
            "save" => run_command(&["vkterm-state", "save-all"], context("close_save")),
            "discard" => run_command(&["vkterm-state", "discard"], context("close_discard")),
            "close" => self.quit(),
            _ => self.closing = None,
        }
    }

    /// De «Abrir reciente»: "here\tRUTA" abre la carpeta en esta pestaña,
    /// después de cerrar la actual; "window\tRUTA", en una ventana nueva (o va
    /// a su pestaña, si ya está abierta).
    fn open_recent(&mut self, payload: &str) {
        match payload.split_once('\t') {
            Some(("here", folder)) => self.close_folder(Some(folder.to_string())),
            Some((_, folder)) => {
                run_command(&["vkterm", "--window", folder], BTreeMap::new());
            }
            None => {}
        }
    }

    /// Termina la sesión, después de guardar la memoria si había cambios. Si
    /// hay una carpeta que abrir después, la deja antes para bin/vkterm, que la
    /// abre en esta pestaña al terminar la sesión.
    fn quit(&mut self) {
        if let Some(next) = self.next_folder.take() {
            run_command(&["vkterm-state", "save", "next", &next], context("next"));
            return;
        }
        let text = self.snapshot();
        if self.restored && text != self.saved_memory {
            run_command(&["vkterm-state", "save", "workspace", &text], context("final"));
        } else {
            quit_zellij();
        }
    }

    fn project_name(&self) -> String {
        self.project.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    // ---------------------------------------------------- barra de estado

    /// Las barras de estado: instancias fijas de este plugin que no se pueden
    /// enfocar (src/status.rs).
    fn status_bar_ids(&self, manifest: &PaneManifest) -> Vec<u32> {
        let panes = manifest.panes.values().flatten();
        let Some(url) = panes.clone().find(|p| p.is_plugin && p.id == self.me).and_then(|p| p.plugin_url.clone()) else {
            return Vec::new();
        };
        panes
            .filter(|p| p.is_plugin && p.id != self.me && !p.is_floating && !p.is_selectable)
            .filter(|p| p.plugin_url.as_ref() == Some(&url))
            .map(|p| p.id)
            .collect()
    }

    fn set_status_bars(&mut self, bars: Vec<u32>) {
        if bars != self.status_bars {
            self.status_bars = bars;
            self.send_status(true);
        }
    }

    fn send_status(&mut self, force: bool) {
        let (branch, notice) = self.scm.bar();
        let payload = format!("{branch}\t{}\t{}\t{notice}", self.mode_name, status::colors(&self.palette));
        if !force && payload == self.sent_status {
            return;
        }
        for id in &self.status_bars {
            pipe_message_to_plugin(
                MessageToPlugin::new("status").with_destination_plugin_id(*id).with_payload(payload.clone()),
            );
        }
        self.sent_status = payload;
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

/// Contexto de `run_command`: el plugin reconoce el resultado por `vkterm`.
fn context(kind: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("vkterm".to_string(), kind.to_string())])
}

/// Otra instancia flotante del plugin (`mode`), como `MessagePlugin … floating
/// true` en config.kdl: Zellij la lanza si no existe.
fn launch_window(name: &str, mode: &str, payload: Option<String>) {
    let mut message = MessageToPlugin::new(name)
        .with_plugin_url("vkterm")
        .with_plugin_config(BTreeMap::from([("mode".to_string(), mode.to_string())]))
        .new_plugin_instance_should_float(true);
    if let Some(payload) = payload {
        message = message.with_payload(payload);
    }
    pipe_message_to_plugin(message);
}

pub(crate) use vkterm::style::fit;

/// Como Git: es binario si hay un byte NUL en los primeros 8 KB.
fn is_binary(path: &Path) -> bool {
    let mut head = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(8192).read_to_end(&mut head))
        .map(|_| head.contains(&0))
        .unwrap_or(false)
}
