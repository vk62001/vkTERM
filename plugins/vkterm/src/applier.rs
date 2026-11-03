//! Cambios caros de uno en uno: aplicar un tema o una fuente recarga Ghostty
//! y vuelve a activar los atajos del IDE (~2 s). Al recorrer el selector con
//! las flechas llegan muchos seguidos: mientras uno está en marcha, el último
//! pedido espera y, al terminar, se aplica si es distinto del que se aplicó.
//! Así nunca hay dos recargas a la vez y siempre queda el último.
//!
//! Lógica pura, sin Zellij: se prueba con `cargo test --lib`.

#[derive(Default)]
pub struct Applier {
    running: bool,
    /// Lo último que se empezó a aplicar.
    started: Option<String>,
    /// Lo último que se pidió.
    wanted: Option<String>,
}

impl Applier {
    /// Pide aplicar `value`. Devuelve lo que hay que empezar ahora, si algo.
    pub fn want(&mut self, value: &str) -> Option<String> {
        self.wanted = Some(value.to_string());
        if self.running {
            return None;
        }
        self.start()
    }

    /// Terminó lo que estaba en marcha. Devuelve lo siguiente, si algo.
    pub fn finished(&mut self) -> Option<String> {
        self.running = false;
        self.start()
    }

    pub fn running(&self) -> bool {
        self.running
    }

    fn start(&mut self) -> Option<String> {
        if self.wanted.is_none() || self.wanted == self.started {
            return None;
        }
        self.running = true;
        self.started = self.wanted.clone();
        self.started.clone()
    }
}

/// Cambios de tamaño de letra (⌘+, ⌘- y ⌘0) de uno en uno: los pasos que
/// llegan mientras uno está en marcha se suman, y ⌘0 descarta los pendientes.
/// Devuelve el argumento de `vkterm-font size`: "+N", "-N" o "0".
#[derive(Default)]
pub struct Steps {
    running: bool,
    pending: i32,
    reset: bool,
}

impl Steps {
    pub fn step(&mut self, delta: i32) -> Option<String> {
        self.pending += delta;
        self.next()
    }

    pub fn reset(&mut self) -> Option<String> {
        self.pending = 0;
        self.reset = true;
        self.next()
    }

    pub fn finished(&mut self) -> Option<String> {
        self.running = false;
        self.next()
    }

    fn next(&mut self) -> Option<String> {
        if self.running {
            return None;
        }
        let arg = if self.reset {
            "0".to_string()
        } else if self.pending != 0 {
            format!("{:+}", self.pending)
        } else {
            return None;
        };
        self.reset = false;
        self.pending = 0;
        self.running = true;
        Some(arg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_pasos_pendientes_se_suman() {
        let mut s = Steps::default();
        assert_eq!(s.step(1), Some("+1".into()));
        assert_eq!(s.step(1), None);
        assert_eq!(s.step(1), None);
        assert_eq!(s.finished(), Some("+2".into()));
        assert_eq!(s.step(-1), None);
        assert_eq!(s.reset(), None);
        assert_eq!(s.finished(), Some("0".into()));
        assert_eq!(s.finished(), None);
        assert_eq!(s.step(-3), Some("-3".into()));
    }

    #[test]
    fn uno_cada_vez_y_gana_el_ultimo() {
        let mut a = Applier::default();
        assert_eq!(a.want("dark-plus"), Some("dark-plus".into()));
        assert_eq!(a.want("light-plus"), None);
        assert_eq!(a.want("monokai"), None);
        assert!(a.running());
        assert_eq!(a.finished(), Some("monokai".into()));
        assert_eq!(a.finished(), None);
        assert!(!a.running());
    }

    #[test]
    fn no_repite_lo_ya_aplicado() {
        let mut a = Applier::default();
        assert_eq!(a.want("nord"), Some("nord".into()));
        assert_eq!(a.want("dracula"), None);
        assert_eq!(a.want("nord"), None);
        // Al terminar, lo pedido es lo que ya se aplicó: nada más que hacer.
        assert_eq!(a.finished(), None);
        assert_eq!(a.want("nord"), None);
        assert_eq!(a.want("dracula"), Some("dracula".into()));
    }
}
