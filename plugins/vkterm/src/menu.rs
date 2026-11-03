//! Menú contextual del editor (clic derecho), como el de VS Code: Cortar,
//! Copiar, Pegar, Seleccionar todo y Paleta de comandos, con el atajo a la
//! derecha. Aquí, las entradas, moverse por ellas y dónde se coloca la
//! ventana (src/contextmenu.rs).
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// `action` lo ejecuta el controlador en el panel del clic (src/main.rs,
    /// `menu_action`): una tecla de `commands::editor_key` o la paleta.
    Item { label: &'static str, keys: &'static str, action: &'static str },
    Separator,
}

const fn item(label: &'static str, keys: &'static str, action: &'static str) -> Entry {
    Entry::Item { label, keys, action }
}

/// El del editor. Sin selección, Cortar y Copiar toman la línea entera, como
/// en VS Code (Cut|CutLine y Copy|CopyLine en Micro).
pub const EDITOR: &[Entry] = &[
    item("Cortar", "⌘X", "cut"),
    item("Copiar", "⌘C", "copy"),
    item("Pegar", "⌘V", "paste"),
    Entry::Separator,
    item("Seleccionar todo", "⌘A", "select_all"),
    Entry::Separator,
    item("Paleta de comandos…", "⌘⇧P", "command_palette"),
];

/// Margen a cada lado de las filas.
pub const PAD: usize = 1;
/// Separación mínima entre la etiqueta y el atajo.
pub const GAP: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    pub entries: &'static [Entry],
    /// Entrada resaltada; al abrirse, ninguna (como VS Code).
    pub selected: Option<usize>,
}

impl Default for Menu {
    fn default() -> Self {
        Menu::new(EDITOR)
    }
}

impl Menu {
    pub fn new(entries: &'static [Entry]) -> Menu {
        Menu { entries, selected: None }
    }

    fn widths(&self) -> (usize, usize) {
        self.entries.iter().fold((0, 0), |(label_width, keys_width), entry| match entry {
            Entry::Item { label, keys, .. } => {
                (label_width.max(label.chars().count()), keys_width.max(keys.chars().count()))
            }
            Entry::Separator => (label_width, keys_width),
        })
    }

    /// Ancho del atajo más largo, para alinearlos a la derecha.
    pub fn keys_width(&self) -> usize {
        self.widths().1
    }

    /// Tamaño de la ventana, con el marco (una celda por lado).
    pub fn size(&self) -> (usize, usize) {
        let (label_width, keys_width) = self.widths();
        (2 + 2 * PAD + label_width + GAP + keys_width, 2 + self.entries.len())
    }

    /// La entrada de la fila `row` del interior, si se puede elegir.
    pub fn at(&self, row: usize) -> Option<usize> {
        matches!(self.entries.get(row), Some(Entry::Item { .. })).then_some(row)
    }

    /// ↓: la siguiente entrada, sin separadores y dando la vuelta; sin
    /// ninguna resaltada, la primera.
    pub fn down(&mut self) {
        self.step(1);
    }

    /// ↑: la anterior; sin ninguna resaltada, la última.
    pub fn up(&mut self) {
        self.step(self.entries.len().saturating_sub(1));
    }

    fn step(&mut self, by: usize) {
        let count = self.entries.len();
        if count == 0 {
            return;
        }
        let mut i = match self.selected {
            Some(i) => (i + by) % count,
            None if by == 1 => 0,
            None => count - 1,
        };
        for _ in 0..count {
            if self.at(i).is_some() {
                self.selected = Some(i);
                return;
            }
            i = (i + by) % count;
        }
    }

    /// El ratón encima de una fila: la resalta si es una entrada.
    pub fn hover(&mut self, row: usize) {
        if let Some(i) = self.at(row) {
            self.selected = Some(i);
        }
    }

    /// La acción de la entrada resaltada.
    pub fn action(&self) -> Option<&'static str> {
        match self.entries.get(self.selected?) {
            Some(Entry::Item { action, .. }) => Some(action),
            _ => None,
        }
    }
}

/// Esquina superior izquierda de una ventana de `size` (columnas, filas) para
/// un clic en la celda `click`, dentro de la zona de paneles `area`. Como en
/// VS Code, junto al puntero, abajo a la derecha; si no cabe, a la izquierda
/// o arriba, y si tampoco, pegada al borde. Sin tapar la celda del clic.
pub fn position(click: (usize, usize), size: (usize, usize), area: (usize, usize)) -> (usize, usize) {
    let axis = |at: usize, len: usize, total: usize| {
        if at + 1 + len <= total {
            at + 1
        } else if at >= len {
            at - len
        } else {
            total.saturating_sub(len)
        }
    };
    (axis(click.0, size.0, area.0), axis(click.1, size.1, area.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tamano_con_marco_margen_y_atajos() {
        let menu = Menu::default();
        // "Paleta de comandos…" (19) + 4 + "⌘⇧P" (3), más margen y marco.
        assert_eq!(menu.size(), (2 + 2 + 19 + 4 + 3, 2 + 7));
        assert_eq!(menu.keys_width(), 3);
    }

    #[test]
    fn flechas_saltan_los_separadores_y_dan_la_vuelta() {
        let mut menu = Menu::default();
        menu.down();
        assert_eq!(menu.action(), Some("cut"));
        menu.up();
        assert_eq!(menu.action(), Some("command_palette"));
        menu.up();
        assert_eq!(menu.action(), Some("select_all"));
        menu.up();
        assert_eq!(menu.action(), Some("paste"));
        menu.down();
        menu.down();
        assert_eq!(menu.action(), Some("command_palette"));
        menu.down();
        assert_eq!(menu.action(), Some("cut"));

        let mut menu = Menu::default();
        menu.up();
        assert_eq!(menu.action(), Some("command_palette"));
    }

    #[test]
    fn el_raton_resalta_solo_entradas() {
        let mut menu = Menu::default();
        assert_eq!(menu.action(), None);
        menu.hover(4);
        assert_eq!(menu.action(), Some("select_all"));
        menu.hover(3);
        menu.hover(40);
        assert_eq!(menu.action(), Some("select_all"));
        assert_eq!(menu.at(5), None);
        assert_eq!(menu.at(2), Some(2));
    }

    #[test]
    fn se_coloca_junto_al_puntero_o_al_otro_lado_si_no_cabe() {
        assert_eq!(position((10, 5), (30, 9), (120, 40)), (11, 6));
        // Cerca del borde derecho y del de abajo: a la izquierda y arriba.
        assert_eq!(position((110, 38), (30, 9), (120, 40)), (80, 29));
        // Justo cabe.
        assert_eq!(position((89, 30), (30, 9), (120, 40)), (90, 31));
        assert_eq!(position((90, 31), (30, 9), (120, 40)), (60, 22));
        // No cabe a ningún lado: pegada al borde.
        assert_eq!(position((5, 3), (30, 9), (20, 6)), (0, 0));
        assert_eq!(position((15, 2), (30, 9), (40, 6)), (10, 0));
    }

    #[test]
    fn toda_accion_tiene_su_tecla_o_es_la_paleta() {
        for entry in EDITOR {
            if let Entry::Item { action, .. } = entry {
                assert!(
                    *action == "command_palette" || crate::commands::editor_key(action).is_some(),
                    "{action}"
                );
            }
        }
    }
}
