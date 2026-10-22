# Plugins de vkTERM (Rust + WebAssembly)

## vkterm

`plugins/vkterm/`: explorador tipo VS Code y controlador del workspace. Detalle
en [docs/architecture.md](../docs/architecture.md#plugin-vkterm).

- Crate `zellij-tile` de la misma versión que el Zellij instalado (0.45.1).
- Compila a `wasm32-wasip1` (`.cargo/config.toml`).
- La lógica pura (`src/tree.rs`, `src/layout.rs`, `src/shortcuts.rs`,
  `src/themes.rs`) está en una biblioteca que se prueba en el Mac.
  `src/main.rs` es el plugin; `src/overlay.rs`, la ventana de atajos;
  `src/picker.rs`, el selector de tema, y `src/floating.rs`, lo común a las
  dos ventanas.
- `config/zellij/config.kdl` lo carga con el alias `vkterm`:
  `file:/$VKTERM_HOME/plugins/vkterm/target/wasm32-wasip1/release/vkterm.wasm`.
  Opciones: `inactive_fg` e `inactive_bg` (`#rrggbb`), los colores de los
  paneles sin foco (por defecto, los del tema). Con `mode "shortcuts"` la
  instancia es la ventana de atajos (⌘K ⌘S) y con `mode "themes"`, el selector
  de tema (⌘K ⌘T), los dos en un panel flotante.

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
