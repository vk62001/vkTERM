//! Lo común a las ventanas flotantes del plugin (atajos, temas, paleta y
//! confirmación al cerrar la carpeta): se abren y se cierran con el mismo
//! atajo, toman el foco, se centran en la zona de paneles con un marco propio
//! y avisan cuando pierden el foco.
use zellij_tile::prelude::*;

/// Marco propio, con el título arriba y la ayuda abajo. El de Zellij añade
/// "PIN [ ]" y la ayuda para redimensionar.
pub const FRAME: usize = 1;

#[derive(Default)]
pub struct Floating {
    pub me: u32,
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
        let mut events = vec![EventType::Key, EventType::Mouse, EventType::PaneUpdate, EventType::TabUpdate];
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

/// Marco redondeado con `title` centrado arriba y `hint` centrado abajo.
pub fn frame(rows: usize, cols: usize, title: &str, hint: &str) {
    if rows < 2 || cols < 2 {
        return;
    }
    let border = |left: &str, label: &str, right: &str| {
        let label = format!(" {label} ");
        let inner = cols - 2;
        let width = label.chars().count().min(inner);
        let before = (inner - width) / 2;
        let line = format!(
            "{left}{}{}{}{right}",
            "─".repeat(before),
            label.chars().take(width).collect::<String>(),
            "─".repeat(inner - width - before),
        );
        (line, 1 + before..1 + before + width)
    };
    let (top, title) = border("╭", title, "╮");
    let text = Text::new(&top).dim_range(..title.start).color_range(2, title.clone()).dim_range(title.end..);
    print_text_with_coordinates(text, 0, 0, Some(cols), Some(1));
    for y in 1..rows - 1 {
        print_text_with_coordinates(Text::new("│").dim_all(), 0, y, None, None);
        print_text_with_coordinates(Text::new("│").dim_all(), cols - 1, y, None, None);
    }
    let (bottom, _) = border("╰", hint, "╯");
    print_text_with_coordinates(Text::new(&bottom).dim_all(), 0, rows - 1, Some(cols), Some(1));
}
