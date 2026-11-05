# Estudio: rehacer vkTERM en Rust

**Fecha:** 5-11-2026 · **Estado:** estudio, sin decidir · **Versión de vkTERM
en ese momento:** 0.5.0

Este documento guarda lo que se averiguó para poder retomarlo más adelante.
Lo hizo un agente de investigación (solo lectura del repositorio y búsqueda
en la web). Los datos de librerías externas (versiones, licencias, estado)
son de esa búsqueda del 5-11-2026 y **no se verificaron a mano**: compruébalos
antes de decidir. Las cifras de esfuerzo son estimaciones.

## Por qué se planteó

El usuario estaba cansado de errores que reaparecen («vamos a seguir parchando
y será un monstruo»). Los problemas encontrados ese día estaban todos en las
costuras entre Ghostty, Zellij, el plugin y Micro:

| Problema | Dónde estaba | Cómo quedó en 0.5.0 |
|---|---|---|
| El clic derecho no llegaba al explorador ni al editor sin el foco | Zellij 0.45.1 no pasa el clic derecho a los plugins | Parche propio (`patches/zellij/0001`), Zellij compilado con `scripts/build-zellij.sh` |
| ⌃L, `clear`, nano y htop fallaban por ssh desde el Shell del IDE | Los paneles heredaban `TERM=xterm-ghostty` | `env { TERM "xterm-256color" }` en `config/zellij/config.kdl` |
| Los atajos ⌘ morían tras una recarga de Ghostty | Ghostty vacía las tablas de atajos en cada recarga | Vigilante `bin/vkterm-surface keep` (reactiva cada 2 s) |
| Edición › Copiar sin ⌘C | Los atajos `performable` no salen en el menú | `super+c` sin `performable` en el modo terminal (Pegar sigue sin ⌘V) |
| Arrastrar en un panel sin el foco no seleccionaba (⌘C no copiaba) | Zellij solo enfoca en el primer clic | `mouse_click_through true` |
| Ventanas pidiendo «Allow?» | Se vació `~/Library/Caches` (¿CleanMyMac?) y Zellij olvidó los permisos | `bin/vkterm-permissions` (0.4.x) |
| Desapareció el Ghostty compilado | Mismo vaciado de cachés | Las compilaciones van ahora a `~/Library/Application Support/vkterm` |
| El servidor de Zellij moría al arrancar (5 de 19 veces) | `unwrap()` si `list-sessions` consulta una sesión que aún no existe | Parche propio (`patches/zellij/0002`) |
| `zellij list-sessions` a veces tarda más de 5 s mientras arranca otra sesión | Zellij 0.45.1 (también el de Homebrew) | **Sin arreglar** |
| Las teclas sintetizadas no son fiables para probar | macOS / Ghostty | `scripts/e2e-ghostty.py` usa la AppleScript de Ghostty; ⌘C/⌘V/⌘K se prueban con su acción |

Resultado de las pruebas ese día: `scripts/e2e-ghostty.py` (4 ventanas en
vkTERM Test.app) **52/52** con el Zellij parcheado (solo el parche 0001); la
batería PTY (`scripts/smoke-test.py`) **251/251** con los dos parches.

## Lo que hay hoy (medido)

| Pieza | Tamaño |
|---|---|
| Plugin Zellij (Rust → WASM), `plugins/vkterm/src` | 8 990 líneas, 34 módulos |
| · lógica pura, sin API de Zellij, con tests | ~4 060 líneas, 92 tests (tree, git, fuzzy, shortcuts, style, menu, quickpick, memory, closing, commands, layout, themes…) |
| · acoplado a Zellij (main.rs, scm.rs, ventanas flotantes) | ~4 900 líneas |
| Scripts zsh, `bin/` | 2 589 líneas, 17 scripts |
| Configuración (Ghostty, Zellij, Micro + Lua, zsh, mlsp) | ~1 250 líneas |
| Pruebas y herramientas (Python, Swift) | 6 568 líneas |
| Parches a terceros | Ghostty 1; Zellij 2 |

Total propio: unas 19 000 líneas, ~5 000 de pruebas. 70 atajos ⌘ en la tabla
`vkterm` de Ghostty; 24 temas generados desde los de VS Code.

## Lo que una versión nueva no puede perder

| Grupo | Funciones | Dificultad en una app propia |
|---|---|---|
| Terminal diario | pestañas, divisiones, teclado Kitty, selección y copia, OSC 7/52, terminfo por ssh, temas, fuente, menús de macOS, ⌥ del teclado español ISO | **Alta**: hoy lo da Ghostty (Metal, fuentes, IME, menús, Secure Input) |
| Modo IDE por pestaña | `vkterm .`, ⌘K F devuelve el shell, «Traer el IDE aquí», ⌘O recientes, una carpeta = una pestaña, memoria por carpeta | Media |
| Paneles | explorador · editor / Shell · Claude Code / Codex; ocultar y mostrar, ⌘1–⌘5, atenuado sin foco, zoom, flotantes, sesión que sobrevive a la pestaña | Media (la sesión que sobrevive exige un servidor aparte) |
| Explorador y Control de código fuente | árbol con colores git, nuevo / renombrar / menú, ⌃⇧G, stage / commit / pull / push, conflictos con `merge-tree`, diff / blame / historial | Baja: ~1 400 líneas portables |
| Editor | pestañas, multicursor, comentar, indentar, mover / duplicar línea, ⌘F, ⌘⇧F, split, autoguardado, resaltado por tema, LSP, memoria de pestañas, menú contextual | **Alta** si es propio; nula si se mantiene Micro |
| Paleta e interfaz | ⌘P, ⌘⇧P, ⌘K ⌘S, quick picks, confirmaciones, barra de estado | Baja: ~1 800 líneas reutilizables |
| Agentes | Claude Code y Codex, imagen con ⌘V, ⌘K ⌘C | Baja |
| Infraestructura | aislamiento de `~/.config`, LSP con versiones fijas, doctor, DMG, pruebas | Media: las pruebas PTY se rehacen |

Lo que hoy **no** se hace por límites de Micro (plegado, minimapa, migas,
panel de Problemas, depurador, brackets por nivel) se desbloquearía con un
editor propio, pero no es requisito.

## Piezas disponibles en Rust (búsqueda web, 5-11-2026)

| Pieza | Estado | Licencia |
|---|---|---|
| `alacritty_terminal` 0.26.0 | maduro; teclado Kitty, OSC 52; sin imágenes; Zed usa un fork | Apache-2.0 |
| `libghostty-vt` (Ghostty 1.4.0) | API en C pre-1.0 (rompe ABI); sin PTY ni renderizador; imágenes Kitty; bindings Rust 0.2.2 que compilan Ghostty con Zig | MIT |
| `termwiz` / `wezterm-term` | termwiz publicado; el emulador solo por git | MIT |
| `vt100` 0.18.1 | parser y pantalla para multiplexores; sin imágenes | MIT |
| GPUI 0.2.2 (de Zed) | Metal, IME (`NSTextInputClient`), accesibilidad en curso, cambios de API frecuentes | Apache-2.0 |
| egui 0.37 / iced 0.14 | egui con AccessKit; iced «experimental» | MIT / Apache |
| WezTerm | sin versión estable desde feb-2024 (nightly activo); multiplexor, tablas de atajos, Lua, pero la API Lua no permite paneles ni menús propios | MIT |
| Zed | editor, terminal, LSP, git, paleta, agentes; extensiones sin interfaz propia | GPL-3.0 (gpui Apache) |
| helix-core | solo por git | MPL-2.0 |
| ropey, tree-sitter, async-lsp | activos | MIT / Apache |
| Ghostty 1.4.0 | sigue sin opción para ocultar el indicador de tablas: el parche seguiría haciendo falta | MIT |
| Zellij | sin versión publicada que pase el clic derecho a los plugins | MIT |

## Opciones

| Opción | Qué desaparece de raíz | Qué queda | Coste estimado |
|---|---|---|---|
| **A.** App nativa: alacritty_terminal + GPUI (o egui) + PTY y paneles propios | todos los problemas de la tabla de arriba | sin imágenes Kitty; fuentes, IME, accesibilidad y menús pasan a ser nuestros | 4–6 meses con Micro; 8–12 con editor propio |
| **B.** App nativa sobre libghostty-vt | igual que A, con imágenes y la fidelidad de Ghostty | API pre-1.0, compilación con Zig; el renderizador sigue siendo nuestro | A + ~2 semanas |
| **C.** Fork de WezTerm | TERM, permisos, carrera de arranque, selección, tablas de atajos | menús y paneles propios exigen modificar su interfaz; sin versión estable | 3–5 meses |
| **D.** Encima de Zed | ya trae editor, LSP, terminal, git, paleta, temas, agentes | no es un terminal diario que se convierte; mantener un fork de un repositorio enorme | 1–2 meses + mantenimiento continuo |

**Observación clave:** lo que elimina los errores no es «Rust», sino que **un
solo proceso sea dueño del teclado, del ratón y de los paneles**.

## Esfuerzo (estimación)

Supuestos: un desarrollador con asistente de IA, ~25 h/semana efectivas, Rust
intermedio, solo macOS en Apple Silicon, «igualar lo de hoy» sin extras.

| Fase | Contenido | A / B |
|---|---|---|
| 1. Núcleo terminal | ventana, pestañas, PTY, VT, texto con fallback, selección, portapapeles, teclado Kitty, menús, IME e ISO, tema y fuente | 6–8 semanas (B: +2) |
| 2. Paneles e IDE | disposición tipo VS Code, ocultar y mostrar, foco, atenuado, memoria, ⌘K F, recientes, «Traer el IDE aquí» | 4–6 semanas |
| 3. Explorador, Control de código fuente, paleta, atajos, barra | portar las ~4 000 líneas puras y rehacer las vistas | 3–5 semanas |
| 4. Editor | **4a** Micro en un PTY: 1–2 semanas · **4b** propio (tree-sitter + LSP) hasta igualar a Micro: 10–16 semanas | |
| 5. Agentes, ssh, DMG, firma, doctor, pruebas | | 3–4 semanas |
| **Total** | con Micro **4–6 meses**; con editor propio **8–12 meses** | |

## Riesgos y lo que se pierde

- **Fuentes y rendimiento:** Ghostty está muy optimizado; ligaduras, emoji,
  fallback y cursores pasan a ser trabajo propio.
- **Teclado e IME:** GPUI usa `NSTextInputClient` (las teclas muertas y ⌥ del
  teclado español deberían funcionar); con winit / egui es menos seguro.
  Probar pronto con el teclado físico.
- **Accesibilidad:** VoiceOver peor que hoy durante meses.
- **macOS:** Secure Input, Services, permisos TCC, notarización: todo propio.
- **Mantenimiento:** de 3 parches pequeños a ser dueños de un terminal
  completo; GPUI y libghostty-vt cambian de API; se pierden las mejoras de
  Ghostty.
- **Pruebas:** se gana (un modelo que se prueba sin AppleScript), pero
  `smoke-test.py` (2 700 líneas) se descarta.
- **Sesiones vivas:** «Traer el IDE aquí» con los agentes vivos exige separar
  un servidor (los PTY) de la ventana, como Zellij. Es el punto de diseño más
  delicado.

## Recomendación: incremental, no de golpe

1. **Ya (0–2 semanas):** congelar versiones (hecho en 0.5.0: Zellij y
   Ghostty parcheados, compilaciones fuera de las cachés) y cerrar las pruebas
   manuales pendientes. Sin cambios de arquitectura.
2. **Fase 1 (2–3 meses): sustituir Zellij por un multiplexor propio en Rust**
   (nombre provisional `vkmux`: TUI con ratatui + `portable-pty` + `vt100` o
   `alacritty_terminal` para los paneles) que corre **dentro de Ghostty**.
   Quita de raíz el clic derecho a plugins, los permisos y la caché de Zellij,
   la carrera de `list-sessions`, la selección sin foco, el TERM filtrado y el
   sandbox WASM. Reutiliza la lógica pura del plugin compilada en nativo,
   `bin/vkterm-git`, los temas y Micro + mlsp sin tocarlos. Zellij queda de
   respaldo hasta igualarlo.
3. **Fase 2 (opcional, 3–5 meses más): app nativa** (GPUI + alacritty_terminal,
   o libghostty-vt cuando llegue a 1.0) que reutiliza `vkmux` como modelo.
   Quita lo que queda: la recarga de las tablas de atajos, el menú Edición,
   AppleScript y las pruebas con teclas sintetizadas. Decidirla con lo
   aprendido en la fase 1.
4. **Editor:** seguir con Micro hasta que algo concreto (plegado, brackets por
   nivel, Problemas) justifique los 3–4 meses de uno propio; antes, evaluar
   helix-core como base.

Alternativa barata si importa el producto y no el terminal: Zed configurado
(terminales con Claude Code y Codex, `zed .`). Cubre gran parte del
inventario, pero no es un terminal diario que se convierte en IDE.

## Cómo retomarlo

Primeros pasos propuestos para la fase 1, en orden:

1. **Separar la lógica pura** de `plugins/vkterm/src` en un crate sin
   `zellij-tile` (p. ej. `crates/vkterm-core`), con sus 92 tests. Sirve ya hoy
   (el plugin lo usaría igual) y es la base de `vkmux`.
2. **Prototipo de 1–2 semanas** de `vkmux` dentro de Ghostty: dos paneles con
   PTY, foco con el ratón, clic derecho que abre un menú, selección que copia
   por OSC 52, `TERM` fijo y ⌘1/⌘2 desde la tabla de atajos de Ghostty. Si
   esto no sale limpio, replantear.
3. **Decidir** antes de seguir:
   - ¿la sesión debe sobrevivir a la pestaña («Traer el IDE aquí»)? Si sí,
     servidor por socket desde el principio;
   - `vt100` o `alacritty_terminal` para los paneles;
   - cómo se prueban las teclas (modelo en Rust con tests, sin AppleScript).
4. Mantener `scripts/e2e-ghostty.py` como prueba de aceptación: lo que pasa
   hoy (52/52) debe seguir pasando con `vkmux`.

## Fuentes

Locales: README.md, docs/architecture.md, docs/keybindings.md,
docs/next-steps.md, CHANGELOG.md (0.5.0), plugins/vkterm/Cargo.toml,
patches/, THIRD_PARTY_NOTICES.md. Web (5-11-2026): crates.io
(`alacritty_terminal`, `termwiz`, `vt100`, `gpui`, `egui`, `iced`, `winit`,
`accesskit_macos`, `async-lsp`, `ropey`, `tree-sitter`, `helix-core`);
ghostty.org/docs/install/release-notes/1-4-0 e `include/ghostty/vt.h`;
github.com/Uzaaft/libghostty-rs; github.com/longbridge/gpui-component;
github.com/wezterm/wezterm/releases y wezterm.org/config/lua;
github.com/zed-industries/zed (README y `crates/terminal/Cargo.toml`);
zed.dev/docs/extensions; github.com/zellij-org/zellij/releases;
github.com/zyedidia/micro/releases; github.com/iced-rs/iced/issues/552;
github.com/helix-editor/helix/issues/42.
