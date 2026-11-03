# Plugins de vkTERM (Rust + WebAssembly)

## vkterm

`plugins/vkterm/`: explorador tipo VS Code y controlador del workspace. Detalle
en [docs/architecture.md](../docs/architecture.md#plugin-vkterm).

- Crate `zellij-tile` de la misma versión que el Zellij instalado (0.45.1).
- Compila a `wasm32-wasip1` (`.cargo/config.toml`).
- La lógica pura (`src/tree.rs`, `src/layout.rs`, `src/shortcuts.rs`,
  `src/themes.rs`, `src/style.rs`, `src/quickpick.rs`, `src/applier.rs`…)
  está en una biblioteca que se prueba en el Mac. `src/main.rs` es el plugin;
  `src/overlay.rs`, la ventana de atajos; `src/picker.rs` y `src/fonts.rs`,
  los selectores de tema y de fuente; `src/palette.rs`, la paleta;
  `src/prompt.rs`, la ventana «Nuevo archivo» (⌘N); `src/scm.rs`, la vista
  Control de código fuente (⌃⇧G, con la lógica en `src/git.rs`);
  `src/branches.rs`, la rama con la que comparar, y `src/floating.rs`, lo
  común a las ventanas.
- `src/themes_data.rs` (la tabla de temas) lo genera `scripts/gen-themes.py`:
  no se edita a mano.
- `config/zellij/config.kdl` lo carga con el alias `vkterm`:
  `file:/$VKTERM_HOME/plugins/vkterm/target/wasm32-wasip1/release/vkterm.wasm`.
  Opciones: `inactive_fg` e `inactive_bg` (`#rrggbb`), los colores de los
  paneles sin foco (por defecto, los del tema). Con `mode "shortcuts"` la
  instancia es la ventana de atajos (⌘K ⌘S); con `mode "themes"`, el selector
  de tema (⌘K ⌘T); con `mode "fonts"`, el de fuente (⌘K ⌘L); con
  `mode "recent"`, «Abrir reciente» (⌘O); con `mode "newfile"`, la que
  pide el nombre de un archivo nuevo (⌘N); con `mode "confirm"`, las
  confirmaciones, y con `mode "branches"`, la rama con la que Control de
  código fuente compara los conflictos, en un panel flotante.

```sh
rustup target add wasm32-wasip1        # una vez
scripts/build-plugins.sh               # compilar
scripts/build-plugins.sh --test        # compilar y ejecutar los tests
scripts/build-plugins.sh --if-needed   # lo que hace bin/vkterm al arrancar
```

El `.wasm` y `target/` no se versionan. `Cargo.lock` sí, para fijar las
dependencias.

Para ver los mensajes de error del plugin en una sesión, revisa el log de
Zellij: `$TMPDIR/zellij-$(id -u)/zellij-log/zellij.log`.
