//! ⌘V con una imagen en el portapapeles, para Claude Code y Codex. Los dos la
//! adjuntan con ⌃V leyendo ellos mismos el portapapeles de macOS; ⌘V en un
//! terminal solo puede pegar texto. El controlador (src/main.rs) pregunta a
//! `osascript -e 'clipboard info'` qué hay y, si es solo una imagen, manda ⌃V
//! al agente con el foco.
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.

/// Tipos de imagen de `clipboard info` (capturas, navegadores, Vista Previa).
const IMAGE_TYPES: [&str; 8] = [
    "«class PNGf»",
    "TIFF picture",
    "JPEG picture",
    "GIF picture",
    "«class jp2 »",
    "«class BMP »",
    "«class heic»",
    "«class 8BPS»",
];
/// Tipos de texto: si hay alguno, Ghostty ya lo pega con ⌘V.
const TEXT_TYPES: [&str; 4] = ["«class utf8»", "«class ut16»", "string", "Unicode text"];

/// `clipboard info` es "tipo, tamaño, tipo, tamaño…". true si hay una imagen
/// y ningún texto (un archivo copiado en Finder trae su nombre como texto).
pub fn image_only(info: &str) -> bool {
    let types: Vec<&str> = info.trim().split(", ").step_by(2).collect();
    types.iter().any(|t| IMAGE_TYPES.contains(t)) && !types.iter().any(|t| TEXT_TYPES.contains(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captura_de_pantalla() {
        assert!(image_only("«class PNGf», 140385, «class 8BPS», 412046, GIF picture, 39652, «class jp2 », 120360, JPEG picture, 62124, TIFF picture, 1638518, «class BMP », 1638454, «class TPIC», 146016\n"));
    }

    #[test]
    fn imagen_copiada_de_un_navegador() {
        assert!(image_only("TIFF picture, 1638518, «class PNGf», 140385"));
    }

    #[test]
    fn texto() {
        assert!(!image_only("«class utf8», 5, «class ut16», 12, string, 5, Unicode text, 10"));
    }

    #[test]
    fn archivo_copiado_en_finder() {
        assert!(!image_only("«class furl», 52, «class icns», 79390, «class ut16», 18, «class utf8», 8, Unicode text, 16, string, 8"));
    }

    #[test]
    fn imagen_con_texto() {
        assert!(!image_only("«class PNGf», 140385, «class utf8», 22, string, 22"));
    }

    #[test]
    fn vacio() {
        assert!(!image_only(""));
    }
}
