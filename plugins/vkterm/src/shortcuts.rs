//! Contenido y maquetación de la ventana de atajos (⌘K ⌘S), al estilo del
//! "Keyboard Shortcuts" de Unity (unity/shortcuts): categorías apiladas en
//! columnas de tres, separadores entre ellas y, en cada fila, el atajo a la
//! izquierda y la descripción a la derecha. Cada columna toma el ancho de su
//! texto más largo. Si no caben las columnas, todo va en una sola.
use std::ops::Range;

pub struct Category {
    pub name: &'static str,
    /// (atajo, descripción)
    pub entries: &'static [(&'static str, &'static str)],
}

/// Lo que dice cada atajo debe coincidir con config/ghostty/vkterm.ghostty y
/// config/zellij/config.kdl: scripts/smoke-test.py comprueba que aquí está
/// cada atajo ⌘ del perfil de Ghostty.
pub const CATEGORIES: &[Category] = &[
    Category {
        name: "Paneles",
        entries: &[
            ("⌘B", "Mostrar u ocultar el explorador"),
            ("⌘J", "Mostrar u ocultar el Shell"),
            ("⌥⌘B", "Mostrar u ocultar Claude Code y Codex"),
            ("⌘1  ⌘2", "Foco al explorador / al editor"),
            ("⌘3  ⌘4", "Foco a Claude Code / a Codex"),
            ("⌘5", "Foco al Shell"),
            ("⌘⌥↑  ⌘⌥↓", "Foco al panel de arriba / de abajo"),
            ("⌘K  Z", "Ampliar o restaurar el panel"),
        ],
    },
    Category {
        name: "Editor",
        entries: &[
            ("⌘⌥←  ⌘⌥→", "Pestaña anterior / siguiente"),
            ("⌃⇧Tab  ⌃Tab", "Pestaña anterior / siguiente"),
            ("⌘W", "Cerrar la pestaña"),
            ("⌘P", "Abrir un archivo por nombre (búsqueda difusa)"),
            ("⌘⇧P", "Paleta de comandos"),
            ("⌘S", "Guardar"),
            ("⌘Z  ⌘⇧Z", "Deshacer / rehacer"),
            ("⌘X  ⌘C  ⌘V", "Cortar / copiar / pegar (sin selección, la línea)"),
            ("⌘A", "Seleccionar todo"),
            ("⌘F", "Buscar en el archivo"),
            ("⌘←  ⌘→", "Inicio / fin de línea"),
        ],
    },
    Category {
        name: "Explorador",
        entries: &[
            ("↑  ↓", "Mover la selección"),
            ("→  ←", "Desplegar / plegar la carpeta"),
            ("Enter", "Abrir en una pestaña"),
            ("Espacio", "Abrir sin salir del explorador"),
            ("a … z", "Saltar a la siguiente entrada con esa letra"),
            ("Ctrl h", "Mostrar u ocultar .git, .DS_Store…"),
        ],
    },
    Category {
        name: "Agentes y Shell",
        entries: &[
            ("Enter", "Arrancar el agente en espera"),
            ("Esc", "Abrir un shell en lugar del agente"),
            ("⌘C", "Copiar sin interrumpir el proceso"),
            ("⌘V", "Pegar; en Claude Code y Codex, también una imagen"),
            ("Ctrl c", "Interrumpir el proceso"),
        ],
    },
    Category {
        name: "Zellij",
        entries: &[
            ("Ctrl g", "Modo normal (Esc vuelve a LOCKED)"),
            ("Ctrl g  p", "Paneles: n nuevo, f ampliar"),
            ("Ctrl g  t", "Pestañas: n nueva, x cerrar"),
            ("Ctrl g  r", "Redimensionar con h j k l"),
            ("Ctrl g  s", "Desplazar la salida (f busca)"),
            ("Ctrl g  o d", "Salir dejando la sesión abierta"),
        ],
    },
    Category {
        name: "Teclado",
        entries: &[
            ("⌥2  ⌥3", "Escribe @ y #, con cualquiera de los dos ⌥"),
            ("⌥ + tecla", "Los demás caracteres: [ ] { } | \\ ~"),
            ("⌥←  ⌥→", "Palabra anterior / siguiente en el editor"),
            ("⌘K  ⌘T", "Cambiar el tema de color"),
            ("⌘K  ⌘S", "Mostrar u ocultar esta ventana"),
        ],
    },
];

/// Como Unity (ShortcutModel: categories_per_column = 3).
pub const PER_COLUMN: usize = 3;
/// Columnas de separación entre el atajo y su descripción.
const KEY_GAP: usize = 2;
/// Columnas de separación entre dos columnas de categorías.
const COLUMN_GAP: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Heading,
    Key,
    Rule,
}

/// Una fila de la ventana. Los rangos son posiciones de carácter; todos los
/// caracteres que se usan ocupan una columna.
#[derive(Clone, Debug, Default)]
pub struct Line {
    pub text: String,
    pub spans: Vec<(Style, Range<usize>)>,
}

impl Line {
    pub fn width(&self) -> usize {
        self.text.chars().count()
    }

    fn push(&mut self, text: &str, style: Option<Style>) {
        let start = self.width();
        self.text.push_str(text);
        if let Some(style) = style {
            self.spans.push((style, start..self.width()));
        }
    }

    fn pad_to(&mut self, width: usize) {
        let missing = width.saturating_sub(self.width());
        self.text.push_str(&" ".repeat(missing));
    }
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

/// Filas de una columna de categorías y su ancho.
fn column(categories: &[Category]) -> (Vec<Line>, usize) {
    let entries = categories.iter().flat_map(|c| c.entries.iter());
    let key_width = entries.clone().map(|(key, _)| chars(key)).max().unwrap_or(0);
    let width = entries
        .map(|(_, description)| key_width + KEY_GAP + chars(description))
        .chain(categories.iter().map(|c| chars(c.name)))
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    for (i, category) in categories.iter().enumerate() {
        if i > 0 {
            let mut rule = Line::default();
            rule.push(&"─".repeat(width), Some(Style::Rule));
            lines.extend([Line::default(), rule, Line::default()]);
        }
        let mut heading = Line::default();
        heading.push(category.name, Some(Style::Heading));
        lines.push(heading);
        for (key, description) in category.entries {
            let mut line = Line::default();
            line.push(key, Some(Style::Key));
            line.pad_to(key_width + KEY_GAP);
            line.push(description, None);
            lines.push(line);
        }
    }
    (lines, width)
}

/// Filas de la ventana para un ancho de contenido dado: columnas de
/// PER_COLUMN categorías si caben, si no una sola columna.
pub fn layout(max_width: usize) -> Vec<Line> {
    let columns: Vec<_> = CATEGORIES.chunks(PER_COLUMN).map(column).collect();
    let total = columns.iter().map(|(_, w)| w).sum::<usize>() + COLUMN_GAP * (columns.len() - 1);
    if total > max_width {
        return column(CATEGORIES).0;
    }
    let height = columns.iter().map(|(lines, _)| lines.len()).max().unwrap_or(0);
    (0..height)
        .map(|row| {
            let mut line = Line::default();
            let mut offset = 0;
            for (i, (lines, width)) in columns.iter().enumerate() {
                if i > 0 {
                    offset += COLUMN_GAP;
                }
                line.pad_to(offset);
                if let Some(cell) = lines.get(row) {
                    let start = line.width();
                    line.text.push_str(&cell.text);
                    line.spans
                        .extend(cell.spans.iter().map(|(s, r)| (*s, r.start + start..r.end + start)));
                }
                offset += width;
            }
            line.text = line.text.trim_end().to_string();
            line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(lines: &[Line]) -> String {
        lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn dos_columnas_de_tres_categorias_si_caben() {
        let lines = layout(200);
        assert!(lines[0].text.starts_with("Paneles"));
        assert!(lines[0].text.contains("Agentes y Shell"));
        assert!(lines.iter().all(|l| l.width() <= 200));
        let names: Vec<_> = CATEGORIES.iter().map(|c| c.name).collect();
        assert!(!lines.iter().any(|l| l.text.trim() == names[3]), "Agentes y Shell va en la segunda columna");
    }

    #[test]
    fn una_sola_columna_si_no_caben() {
        let wide = layout(200);
        let width = wide.iter().map(Line::width).max().unwrap();
        let lines = layout(width - 1);
        let headings: Vec<_> = lines
            .iter()
            .filter(|l| l.spans.first().map(|s| s.0) == Some(Style::Heading))
            .map(|l| l.text.as_str())
            .collect();
        assert_eq!(headings, CATEGORIES.iter().map(|c| c.name).collect::<Vec<_>>());
        assert!(lines.len() > wide.len());
    }

    #[test]
    fn estan_todos_los_atajos_y_descripciones() {
        for width in [200, 60] {
            let all = text(&layout(width));
            for category in CATEGORIES {
                for (key, description) in category.entries {
                    assert!(all.contains(key) && all.contains(description), "{key}");
                }
            }
        }
    }

    #[test]
    fn el_atajo_va_resaltado_y_las_descripciones_alineadas() {
        let lines = layout(200);
        let row = lines.iter().find(|l| l.text.starts_with("⌘B ")).unwrap();
        let (style, range) = row.spans[0].clone();
        assert_eq!(style, Style::Key);
        assert_eq!(range, 0..2);
        let column_of = |prefix: &str, description: &str| {
            let line = lines.iter().find(|l| l.text.starts_with(prefix)).unwrap();
            let byte = line.text.find(description).unwrap();
            line.text[..byte].chars().count()
        };
        assert_eq!(column_of("⌘B ", "Mostrar"), column_of("⌘⌥↑", "Foco al panel"));
    }

    #[test]
    fn separadores_entre_categorias_de_la_misma_columna() {
        let lines = column(&CATEGORIES[..2]).0;
        let rules = lines.iter().filter(|l| l.spans.first().map(|s| s.0) == Some(Style::Rule)).count();
        assert_eq!(rules, 1);
    }
}
