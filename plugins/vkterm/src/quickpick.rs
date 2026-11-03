//! Lista para elegir, como el "Quick Pick" de VS Code: lo escrito filtra
//! (búsqueda difusa, src/fuzzy.rs) sin cambiar el orden, los elementos se
//! agrupan bajo un título (los temas: claros, oscuros y alto contraste) y la
//! lista se desplaza para que el elegido siempre se vea. La usan el selector
//! de tema (⌘K ⌘T) y el de fuentes (⌘K ⌘L).
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.
use crate::fuzzy;

/// Una fila de la lista: el título de un grupo o un elemento (su posición en
/// la lista original y los caracteres que coinciden con lo escrito).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Group(&'static str),
    Item { index: usize, matched: Vec<usize> },
}

#[derive(Default)]
pub struct QuickPick {
    labels: Vec<String>,
    groups: Vec<Option<&'static str>>,
    query: String,
    rows: Vec<Row>,
    /// Fila elegida (siempre un elemento) y primera fila a la vista.
    selected: Option<usize>,
    scroll: usize,
    height: usize,
}

impl QuickPick {
    /// `items`: (texto, grupo). Los de un mismo grupo deben ir seguidos.
    pub fn new(items: Vec<(String, Option<&'static str>)>) -> QuickPick {
        let (labels, groups) = items.into_iter().unzip();
        let mut pick = QuickPick { labels, groups, height: usize::MAX, ..QuickPick::default() };
        pick.refresh();
        pick
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.refresh();
        self.selected = self.rows.iter().position(|r| matches!(r, Row::Item { .. }));
        self.scroll = 0;
        self.keep_visible();
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Filas a la vista (desde `scroll`).
    pub fn visible(&self) -> &[Row] {
        let end = self.rows.len().min(self.scroll.saturating_add(self.height));
        &self.rows[self.scroll.min(end)..end]
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn selected_row(&self) -> Option<usize> {
        self.selected
    }

    /// Posición en la lista original del elemento elegido.
    pub fn selected(&self) -> Option<usize> {
        match self.rows.get(self.selected?) {
            Some(Row::Item { index, .. }) => Some(*index),
            _ => None,
        }
    }

    /// Elige el elemento `index` de la lista original, si está a la vista con
    /// el filtro actual.
    pub fn select_item(&mut self, index: usize) {
        if let Some(row) = self.rows.iter().position(|r| matches!(r, Row::Item { index: i, .. } if *i == index)) {
            self.selected = Some(row);
            self.keep_visible();
        }
    }

    /// Elige la fila `row` (por ejemplo, la del clic), si es un elemento.
    pub fn select_row(&mut self, row: usize) -> bool {
        if matches!(self.rows.get(row), Some(Row::Item { .. })) {
            self.selected = Some(row);
            self.keep_visible();
            true
        } else {
            false
        }
    }

    /// Se mueve `delta` elementos (saltando los títulos), sin dar la vuelta.
    pub fn move_by(&mut self, delta: isize) {
        let items: Vec<usize> = (0..self.rows.len()).filter(|&r| matches!(self.rows[r], Row::Item { .. })).collect();
        if items.is_empty() {
            return;
        }
        let current = self.selected.and_then(|s| items.iter().position(|&r| r == s)).unwrap_or(0);
        let target = current.saturating_add_signed(delta).min(items.len() - 1);
        self.selected = Some(items[target]);
        self.keep_visible();
    }

    pub fn first(&mut self) {
        self.move_by(isize::MIN);
    }

    pub fn last(&mut self) {
        self.move_by(isize::MAX);
    }

    /// Filas que caben en la ventana.
    pub fn set_height(&mut self, height: usize) {
        self.height = height.max(1);
        self.keep_visible();
    }

    fn refresh(&mut self) {
        self.rows.clear();
        let mut group = None;
        for (index, label) in self.labels.iter().enumerate() {
            let matched = if self.query.is_empty() {
                Vec::new()
            } else {
                match fuzzy::fuzzy(&self.query, label) {
                    Some(m) => m.indices,
                    None => continue,
                }
            };
            if self.groups[index] != group {
                group = self.groups[index];
                if let Some(title) = group {
                    self.rows.push(Row::Group(title));
                }
            }
            self.rows.push(Row::Item { index, matched });
        }
    }

    /// Desplaza la lista para que se vea el elegido y, si es el primero de su
    /// grupo, también el título.
    fn keep_visible(&mut self) {
        let Some(selected) = self.selected else {
            self.scroll = 0;
            return;
        };
        let top = if selected > 0 && matches!(self.rows[selected - 1], Row::Group(_)) { selected - 1 } else { selected };
        if top < self.scroll {
            self.scroll = top;
        } else if selected >= self.scroll + self.height {
            self.scroll = selected + 1 - self.height;
        }
        let max = self.rows.len().saturating_sub(self.height);
        self.scroll = self.scroll.min(max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick() -> QuickPick {
        QuickPick::new(vec![
            ("Light+".into(), Some("claros")),
            ("Solarized Light".into(), Some("claros")),
            ("Dark+".into(), Some("oscuros")),
            ("Solarized Dark".into(), Some("oscuros")),
            ("Tokyo Night".into(), Some("oscuros")),
        ])
    }

    #[test]
    fn agrupa_y_salta_los_titulos() {
        let mut p = pick();
        assert_eq!(p.rows()[0], Row::Group("claros"));
        assert_eq!(p.rows()[3], Row::Group("oscuros"));
        p.select_item(1);
        p.move_by(1);
        assert_eq!(p.selected(), Some(2));
        p.move_by(-1);
        assert_eq!(p.selected(), Some(1));
        p.last();
        assert_eq!(p.selected(), Some(4));
        p.first();
        assert_eq!(p.selected(), Some(0));
    }

    #[test]
    fn filtrar_deja_las_coincidencias_en_su_orden_y_elige_la_primera() {
        let mut p = pick();
        p.set_query("solar");
        let items: Vec<usize> = p.rows().iter().filter_map(|r| match r {
            Row::Item { index, .. } => Some(*index),
            _ => None,
        }).collect();
        assert_eq!(items, vec![1, 3]);
        assert_eq!(p.selected(), Some(1));
        assert!(matches!(&p.rows()[1], Row::Item { matched, .. } if matched == &vec![0, 1, 2, 3, 4]));
        p.set_query("zzz");
        assert_eq!(p.selected(), None);
        assert!(p.rows().is_empty());
    }

    #[test]
    fn desplaza_para_que_se_vea_el_elegido_y_su_titulo() {
        let mut p = pick();
        p.set_height(3);
        p.select_item(0);
        assert_eq!(p.scroll(), 0);
        p.select_item(4);
        assert_eq!(p.visible().last(), Some(&Row::Item { index: 4, matched: vec![] }));
        p.select_item(2);
        // Primer elemento de "oscuros": también se ve el título.
        assert_eq!(p.visible()[0], Row::Group("oscuros"));
        assert!(!p.select_row(0));
        assert!(p.select_row(1));
        assert_eq!(p.selected(), Some(0));
    }
}
