//! Búsqueda difusa para la paleta (⌘P y ⌘⇧P), al estilo de VS Code: cada
//! carácter de la consulta debe aparecer en orden en el candidato. Puntúan
//! más las coincidencias al principio de una palabra (tras `/`, `-`, `_`,
//! `.`, un espacio o una mayúscula), las consecutivas y las que caen en el
//! nombre del archivo; penaliza la longitud del candidato para que, a igual
//! coincidencia, gane la ruta más corta.
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.

/// Una coincidencia: puntuación (mayor es mejor) y posiciones de carácter
/// del candidato que coinciden, para resaltarlas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub score: i32,
    pub indices: Vec<usize>,
}

const BOUNDARY: i32 = 10;
const CONSECUTIVE: i32 = 12;
const BASENAME: i32 = 20;
const GAP: i32 = 3;

fn is_boundary(chars: &[char], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let previous = chars[i - 1];
    matches!(previous, '/' | '-' | '_' | '.' | ' ' | ':')
        || (chars[i].is_uppercase() && previous.is_lowercase())
}

/// Coincidencia voraz de `query` dentro de `chars[start..]`: cada carácter se
/// toma en su primera posición disponible, de izquierda a derecha. Puntúan más
/// las coincidencias al principio de palabra y las consecutivas; penalizan los
/// huecos. Devuelve las posiciones y la puntuación parcial.
///
/// No salta a un principio de palabra más adelante aunque puntuara más: hacerlo
/// podría dejar sin sitio a los caracteres siguientes (p. ej. la `t` de `notes`
/// saltaría a la de `.ts` y la `e` y la `s` ya no encajarían).
fn greedy(query: &[char], chars: &[char], start: usize) -> Option<(Vec<usize>, i32)> {
    let lower: Vec<char> = chars.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
    let mut indices = Vec::with_capacity(query.len());
    let mut score = 0;
    let mut position = start;
    for &q in query {
        let i = (position..lower.len()).find(|&i| lower[i] == q)?;
        if is_boundary(chars, i) {
            score += BOUNDARY;
        }
        if indices.last() == Some(&i.wrapping_sub(1)) {
            score += CONSECUTIVE;
        } else if let Some(&last) = indices.last() {
            score -= GAP * ((i - last - 1) as i32).min(10);
        }
        indices.push(i);
        position = i + 1;
    }
    Some((indices, score))
}

/// Coincidencia de `query` en `text`, sin distinguir mayúsculas. Con la
/// consulta vacía todo coincide con puntuación 0 y sin posiciones.
pub fn fuzzy(query: &str, text: &str) -> Option<Match> {
    let query: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
    if query.is_empty() {
        return Some(Match { score: 0, indices: Vec::new() });
    }
    let chars: Vec<char> = text.chars().collect();
    let length_penalty = (chars.len() / 8) as i32;
    // Primero dentro del nombre del archivo (lo que va tras la última `/`).
    let basename = chars.iter().rposition(|&c| c == '/').map_or(0, |i| i + 1);
    let in_basename = greedy(&query, &chars, basename).map(|(i, s)| (i, s + BASENAME));
    let anywhere = greedy(&query, &chars, 0);
    let (indices, score) = match (in_basename, anywhere) {
        (Some(b), Some(a)) if b.1 >= a.1 => b,
        (_, Some(a)) => a,
        (Some(b), None) => b,
        (None, None) => return None,
    };
    Some(Match { score: score - length_penalty, indices })
}

/// Índices de `items` que coinciden con `query`, de mejor a peor; a igual
/// puntuación, por orden original (que ya es alfabético para los archivos).
pub fn rank<S: AsRef<str>>(query: &str, items: &[S]) -> Vec<(usize, Match)> {
    let mut matches: Vec<(usize, Match)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| fuzzy(query, item.as_ref()).map(|m| (i, m)))
        .collect();
    matches.sort_by(|a, b| b.1.score.cmp(&a.1.score).then(a.0.cmp(&b.0)));
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order<'a>(query: &str, items: &[&'a str]) -> Vec<&'a str> {
        rank(query, items).into_iter().map(|(i, _)| items[i]).collect()
    }

    #[test]
    fn la_consulta_vacia_devuelve_todo_en_orden() {
        let items = ["b", "a", "c"];
        assert_eq!(order("", &items), vec!["b", "a", "c"]);
        assert_eq!(fuzzy("", "x").unwrap().indices, Vec::<usize>::new());
    }

    #[test]
    fn exige_los_caracteres_en_orden() {
        assert!(fuzzy("hdr", "src/components/Header.tsx").is_some());
        assert!(fuzzy("rdh", "src/components/Header.tsx").is_none());
        assert!(fuzzy("zzz", "src/App.tsx").is_none());
    }

    #[test]
    fn coincide_aunque_la_extension_repita_letras_de_la_consulta() {
        // La `t` de "notes" no debe saltar a la `t` de ".ts".
        let m = fuzzy("notes", "src/notes.ts").expect("notes coincide con src/notes.ts");
        assert_eq!(m.indices, vec![4, 5, 6, 7, 8]);
        assert_eq!(fuzzy("notes", &"src/notes.ts"[4..]).unwrap().indices, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn no_distingue_mayusculas_y_resalta_las_posiciones() {
        let m = fuzzy("HEAD", "src/components/Header.tsx").unwrap();
        assert_eq!(m.indices, vec![15, 16, 17, 18]);
    }

    #[test]
    fn prefiere_el_nombre_del_archivo_y_los_principios_de_palabra() {
        let items = ["src/notes/head-error.ts", "src/components/Header.tsx", "lib/head/er.ts"];
        assert_eq!(order("header", &items)[0], "src/components/Header.tsx");
        let items = ["src/app.config.ts", "src/App.tsx"];
        assert_eq!(order("app", &items)[0], "src/App.tsx");
    }

    #[test]
    fn las_iniciales_de_cada_carpeta_tambien_cuentan() {
        let items = ["docs/phase-1-plan.md", "src/components/Header.tsx"];
        assert_eq!(order("sch", &items)[0], "src/components/Header.tsx");
    }

    #[test]
    fn ignora_los_espacios_de_la_consulta() {
        assert!(fuzzy("comp header", "src/components/Header.tsx").is_some());
    }
}
