//! Lo común a las ventanas flotantes del plugin (atajos, temas, fuentes,
//! paleta y confirmación al cerrar la carpeta): se abren y se cierran con el
//! mismo atajo, toman el foco, se centran en la zona de paneles con un marco
//! propio, se pintan con los colores del tema (los de la paleta de VS Code,
//! src/style.rs) y avisan cuando pierden el foco.
use vkterm::style::{Line, Palette, Rgb, Slot, Slots};
use zellij_tile::prelude::*;

/// Marco propio, con el título arriba y la ayuda abajo. El de Zellij añade
/// "PIN [ ]" y la ayuda para redimensionar.
pub const FRAME: usize = 1;

#[derive(Default)]
pub struct Floating {
    pub me: u32,
    /// Colores del tema, de cada ModeUpdate.
    pub palette: Palette,
    opened: bool,
    was_focused: bool,
    /// Pestaña en la que está la ventana.
    tab: usize,
    /// Zona de paneles (columnas, filas), según el último PaneUpdate.
    area: (usize, usize),
    /// Zona y tamaño con los que se colocó la ventana.
    placed: (usize, usize, usize, usize),
}

impl Floating {
    pub fn load(&mut self, extra_events: &[EventType]) {
        // Los mismos que el explorador: Zellij guarda en su caché los últimos
        // pedidos por el plugin y, si fueran menos, el explorador volvería a
        // preguntar en la siguiente sesión.
        request_permission(&crate::PERMISSIONS);
        let mut events =
            vec![EventType::Key, EventType::Mouse, EventType::PaneUpdate, EventType::TabUpdate, EventType::ModeUpdate];
        events.extend_from_slice(extra_events);
        subscribe(&events);
        self.me = get_plugin_ids().plugin_id;
    }

    /// El atajo manda el mismo mensaje para abrir y para cerrar: true la
    /// primera vez, cuando la ventana se muestra y toma el foco (un plugin
    /// lanzado por un mensaje no lo recibe).
    pub fn open(&mut self, title: &str) -> bool {
        if self.opened {
            return false;
        }
        self.opened = true;
        rename_plugin_pane(self.me, title);
        focus_plugin_pane(self.me, true, false);
        true
    }

    pub fn area(&self) -> (usize, usize) {
        self.area
    }

    /// Sigue el tema: true si cambió (hay que volver a dibujar).
    pub fn theme(&mut self, event: &Event) -> bool {
        match event {
            Event::ModeUpdate(mode) => {
                let palette = palette(&mode.style);
                let changed = palette != self.palette;
                self.palette = palette;
                changed
            }
            _ => false,
        }
    }

    /// Sigue el foco y la zona de paneles; true si la ventana perdió el foco.
    /// `is_focused` es por capa: al enfocar un panel fijo, Zellij oculta la
    /// capa flotante y la ventana sigue marcada como enfocada en ella.
    pub fn lost_focus(&mut self, event: &Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => {
                self.was_focused && tabs.iter().any(|t| t.position == self.tab && !t.are_floating_panes_visible)
            }
            Event::PaneUpdate(manifest) => {
                let me = self.me;
                let is_me = |p: &PaneInfo| p.is_plugin && p.id == me;
                let Some((tab, panes)) = manifest.panes.iter().find(|(_, ps)| ps.iter().any(is_me)) else {
                    return false;
                };
                self.tab = *tab;
                // Zona de paneles: los fijos visibles, sin la barra inferior.
                // Una sola fuente de tamaño: recolocar provoca otro PaneUpdate.
                let tiled = panes.iter().filter(|p| !p.is_floating && !p.is_suppressed && p.is_selectable);
                let columns = tiled.clone().map(|p| p.pane_x + p.pane_columns).max().unwrap_or(0);
                let rows = tiled.map(|p| p.pane_y + p.pane_rows).max().unwrap_or(0);
                self.area = (columns, rows);
                if panes.iter().any(|p| is_me(p) && p.is_focused) {
                    self.was_focused = true;
                    false
                } else {
                    self.was_focused
                }
            }
            _ => false,
        }
    }

    /// Centra la ventana, sin el marco de Zellij, con ese tamaño exterior.
    pub fn place(&mut self, width: usize, height: usize) {
        let (columns, rows) = self.area;
        if columns == 0 || rows == 0 || self.placed == (columns, rows, width, height) {
            return;
        }
        self.placed = (columns, rows, width, height);
        let (width, height) = (width.min(columns), height.min(rows));
        let coordinates = FloatingPaneCoordinates::new(
            Some(((columns - width) / 2).to_string()),
            Some(((rows - height) / 2).to_string()),
            Some(width.to_string()),
            Some(height.to_string()),
            None,
            Some(true),
        );
        if let Some(coordinates) = coordinates {
            change_floating_panes_coordinates(vec![(PaneId::Plugin(self.me), coordinates)]);
        }
    }
}

/// El explorador (controlador) de la sesión: el panel fijo del mismo plugin
/// que se puede enfocar. La barra de estado también es este plugin, pero no
/// se puede enfocar.
pub fn controller(manifest: &PaneManifest, me: u32) -> Option<u32> {
    let panes = manifest.panes.values().flatten();
    let url = panes.clone().find(|p| p.is_plugin && p.id == me).and_then(|p| p.plugin_url.clone())?;
    panes
        .filter(|p| p.is_plugin && !p.is_floating && p.is_selectable && p.id != me)
        .find(|p| p.plugin_url.as_ref() == Some(&url))
        .map(|p| p.id)
}

/// Marco redondeado con `title` centrado arriba y `hint` centrado abajo (sin
/// nada, si están vacíos), y el interior con el fondo de la paleta
/// (quickInput.background). Se dibuja antes que el contenido.
pub fn frame(rows: usize, cols: usize, title: &str, hint: &str, palette: &Palette) {
    if rows < 2 || cols < 2 {
        return;
    }
    let edge = palette.quick.with_fg(palette.border);
    let border = |left: &str, label: &str, right: &str| {
        let label = if label.is_empty() { String::new() } else { format!(" {label} ") };
        let inner = cols - 2;
        let width = label.chars().count().min(inner);
        let before = (inner - width) / 2;
        let text = format!(
            "{left}{}{}{}{right}",
            "─".repeat(before),
            label.chars().take(width).collect::<String>(),
            "─".repeat(inner - width - before),
        );
        (Line::new(&text, cols, edge), 1 + before..1 + before + width)
    };
    let (mut top, title) = border("╭", title, "╮");
    top.paint(title, |_| palette.quick.bold());
    print!("{}", top.render(0, 0));
    let mut side = Line::new(&format!("│{}│", " ".repeat(cols - 2)), cols, palette.quick);
    side.fg(0..1, palette.border);
    side.fg(cols - 1..cols, palette.border);
    for y in 1..rows - 1 {
        print!("{}", side.render(0, y));
    }
    let (mut bottom, hint) = border("╰", hint, "╯");
    bottom.fg(hint, palette.quick_description);
    print!("{}", bottom.render(0, rows - 1));
}

/// Una fila del interior de una ventana, sobre su fondo.
pub fn row(text: &str, width: usize, palette: &Palette) -> Line {
    Line::new(text, width, palette.quick)
}

/// La línea de lo escrito: `prompt`, el texto y el cursor; sin texto, la
/// ayuda.
pub fn input_line(prompt: &str, query: &str, placeholder: &str, width: usize, palette: &Palette) -> Line {
    let text = if query.is_empty() { format!("{prompt} ▏{placeholder}") } else { format!("{prompt} {query}▏") };
    let mut line = row(&text, width, palette);
    line.fg(0..1, palette.accent);
    if query.is_empty() {
        line.fg(3..width, palette.quick_description);
    }
    line
}

/// Título de un grupo: el nombre y una raya hasta el final.
pub fn group_line(title: &str, width: usize, palette: &Palette) -> Line {
    let label = format!("{title} ");
    let rule = "─".repeat(width.saturating_sub(label.chars().count()));
    let mut line = row(&format!("{label}{rule}"), width, palette);
    let len = label.chars().count();
    line.fg(0..len, palette.group);
    line.fg(len..width, palette.border);
    line
}

/// La paleta de la interfaz a partir del tema de Zellij (config/zellij/themes).
pub fn palette(style: &Style) -> Palette {
    Palette::from_slots(&slots(&style.colors))
}

fn rgb(color: PaletteColor) -> Rgb {
    match color {
        PaletteColor::Rgb((r, g, b)) => Rgb(r, g, b),
        PaletteColor::EightBit(n) => Rgb::from_ansi256(n),
    }
}

fn slot(d: &StyleDeclaration) -> Slot {
    // "background 0" en el tema: sin fondo propio.
    let background = match d.background {
        PaletteColor::EightBit(0) => None,
        other => Some(rgb(other)),
    };
    Slot {
        base: rgb(d.base),
        background,
        emphasis: [rgb(d.emphasis_0), rgb(d.emphasis_1), rgb(d.emphasis_2), rgb(d.emphasis_3)],
    }
}

fn slots(s: &Styling) -> Slots {
    Slots {
        text_unselected: slot(&s.text_unselected),
        text_selected: slot(&s.text_selected),
        table_title: slot(&s.table_title),
        table_cell_selected: slot(&s.table_cell_selected),
        table_cell_unselected: slot(&s.table_cell_unselected),
        list_unselected: slot(&s.list_unselected),
        list_selected: slot(&s.list_selected),
        ribbon_selected: slot(&s.ribbon_selected),
        frame_selected: slot(&s.frame_selected),
        frame_unselected: s.frame_unselected.as_ref().map(slot),
        exit_code_error: slot(&s.exit_code_error),
    }
}
