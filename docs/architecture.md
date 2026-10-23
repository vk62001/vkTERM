# Arquitectura de vkTERM

vkTERM combina componentes estables sin forks: Ghostty, Zellij y Micro. A eso
añade un plugin propio de Zellij, `vkterm` (Rust → WebAssembly), que hace de
explorador y de controlador del workspace, y una app de macOS, vkTERM.app,
que es una copia local de Ghostty.

## Vista general

```
vkTERM.app (copia de Ghostty.app; lanzador scripts/vkterm-app-launcher.c)
 │  perfil config/ghostty/vkterm.ghostty: tabla por defecto (modo terminal) + tabla `vkterm` (modo IDE)
 └─ /usr/bin/login → zsh -l del usuario            ZDOTDIR=config/zsh: integración de vkTERM
     │                                              MODO TERMINAL
     └─ `vkterm .` o ⌘B/⌘J/⌘P/⌘⇧P en el prompt
         └─ bin/vkterm (despachador)               activa la tabla `vkterm` en su pestaña (AppleScript)
             └─ bin/vkterm-workspace <proyecto>     MODO IDE: compila el plugin si hace falta
                 └─ zellij client ⇄ zellij server   --config-dir config/local/zellij/<huella>
                     ├─ Explorador 20%     plugin vkterm (plugins/vkterm): árbol + controlador
                     ├─ Editor             bin/vkterm-editor → micro (pestañas, plugin Lua de memoria)
                     ├─ Shell (abajo 30%)  bin/vkterm-shell → $SHELL
                     ├─ Barra derecha 25%
                     │   ├─ Claude Code    claude  (start_suspended: espera Enter)
                     │   └─ Codex          codex   (start_suspended: espera Enter)
                     └─ Barra de estado    plugin vkterm (mode "status")
```

Al cerrar la carpeta (⌘K F) o la pestaña, la sesión termina y bin/vkterm
desactiva la tabla: vuelve el shell de antes, con su scroll e historial.

```
┌──────────┬────────────────────────┬────────────┐
│ DEMO     │ Header.tsx  [App.tsx]  │ Claude Code│
│ ▾ src    │                        │            │
│   ▸ comp │  Micro                 ├────────────┤
│   App.tsx│                        │ Codex      │
│          ├────────────────────────┤            │
│   ⌘B     │ Shell            ⌘J    │      ⌥⌘B   │
└──────────┴────────────────────────┴────────────┘
```

Cada panel de terminal es un proceso hijo del servidor de Zellij con su propio
PTY.

## Modo terminal y modo IDE

### vkTERM.app (`bin/vkterm-app`)

Una copia de /Applications/Ghostty.app en ~/Applications con:

- **Identificador propio** (`io.github.vk62001.vkterm`), nombre e icono. Hace
  falta para AppleScript: con el de Ghostty, `tell application id` podría
  dirigirse al Ghostty personal del usuario, que tiene otra configuración.
- **Lanzador** como ejecutable principal (`scripts/vkterm-app-launcher.c`):
  macOS lanza las apps sin argumentos, así que el lanzador sustituye su
  proceso por el `ghostty` real con `--config-default-files=false` y el
  perfil de vkTERM. Exporta `VKTERM_HOME` (clave `VKTermHome` del
  Info.plist), `VKTERM_APP_ID` y `ZDOTDIR=config/zsh` (y el ZDOTDIR original
  en `VKTERM_USER_ZDOTDIR`).
- **Info.plist**: sin `CFBundleIconName` (el icono del catálogo de Ghostty
  tendría prioridad), sin su plugin del Dock ni sus servicios, y con las
  actualizaciones automáticas apagadas (además de `auto-update = off`).
- **Firma local** (ad hoc) con los mismos *entitlements*, y **sin la marca de
  cuarentena** que la copia hereda de la Ghostty.app descargada: con la firma
  original modificada, Gatekeeper mataba el proceso.
- Una huella (`VKTermBuild`) de la versión de Ghostty, el lanzador, el icono y
  el propio script: `install` no rehace la app si no cambia nada, y macOS
  conserva los permisos concedidos a esa firma. `install --test` crea
  `vkTERM Test.app` en ~/Library/Caches/vkterm para las pruebas.

### Tablas de atajos (Ghostty 1.3)

El modo de cada pestaña es una tabla de atajos de Ghostty
(`keybind = vkterm/…`). Ghostty busca primero en la tabla activa y después
en la de por defecto. Comprobado con eventos sintetizados:

- las tablas son **por pestaña** (por superficie);
- las teclas con menú de macOS (⌘W, ⌘Z, ⌘A, ⌘F, ⌘C, ⌘V, ⌘D…) llegan a la
  tabla, no al menú, **salvo ⌘T y ⌘⇧,** (New Tab, Reload Configuration):
  macOS las resuelve por el menú. Por eso la tabla por defecto las enlaza por
  tecla física (`super+key_t`, `super+shift+comma`), que no pone atajo en el
  menú;
- **recargar la configuración vacía las tablas**. `bin/vkterm-theme` (cambio
  de tema y ⌘⇧,) las vuelve a activar con `bin/vkterm-surface reactivate`;
  desde el menú de Ghostty no;
- mientras la tabla está activa, Ghostty dibuja «⌨ vkterm» abajo en el
  centro (no se puede ocultar): la barra de estado deja ese hueco libre.

### `vkterm` (bin/vkterm)

```
vkterm [--then ACCIÓN] [--here] [--new] [DIR] [ARCHIVO]
 ├─ ¿la carpeta ya está abierta? (projects/<huella>/surface, pid vivo y la
 │   pestaña existe)                       → bin/vkterm-surface focus; fin
 ├─ --here                                 → bin/vkterm-workspace en este terminal
 ├─ desde Zellij, otro terminal o sin VKTERM_APP_ID
 │                                         → bin/vkterm-surface open: ventana nueva
 └─ en una pestaña de vkTERM.app (modo terminal):
     ├─ OSC 2 con un título único; `vkterm-surface claim` busca ese terminal
     │   por AppleScript, activa la tabla `vkterm` y devuelve su id
     ├─ escribe surface (id, pid, app) y OSC 7 con el proyecto (⌘T abre ahí)
     ├─ bin/vkterm-workspace como proceso hijo (no exec)
     └─ al terminar: desactiva la tabla, borra surface y devuelve OSC 7
```

- **Ventana nueva convertida** (`vkterm-surface open`): con la app abierta,
  AppleScript `new window` con `initial working directory` y la variable
  `VKTERM_AUTOSTART=nonce:acción:carpeta`; con la app cerrada,
  `open -b <id> --env VKTERM_AUTOSTART=…`. La integración de zsh ejecuta
  `vkterm` en su primer prompt; solo la pestaña que reclama el nonce
  (`mkdir`), porque con la app cerrada la variable llega a todas.
- **AppleScript**: el id de la app va escrito en el texto del script. Con
  `tell application id <variable>`, AppleScript no compila los términos de
  Ghostty (`terminal`, `perform action`).

### Integración de zsh (config/zsh)

- `.zshenv` es un puente, como el de la integración de Ghostty (que deja
  ZDOTDIR en config/zsh antes de cargarlo): devuelve ZDOTDIR a su valor,
  carga el `.zshenv` del usuario y registra un `precmd` de un solo uso que
  carga `vkterm.zsh` después del `.zshrc` del usuario. Probado con
  oh-my-zsh y powerlevel10k.
- `vkterm.zsh`: la función `vkterm` (aunque no esté en el PATH); los widgets
  de ⌘B, ⌘J, ⌘P y ⌘⇧P (F13–F16 con Super), que guardan lo escrito
  (`push-line`) y ejecutan `vkterm --then ACCIÓN .`, en los mapas emacs,
  viins y vicmd; y `VKTERM_AUTOSTART`.

### Sesiones y memoria

- **La sesión dura lo que su pestaña.** `on_force_close "quit"` en
  config.kdl: al cerrar la pestaña o la ventana (SIGHUP al cliente) Zellij
  termina la sesión. ⌘K F la termina con `quit_zellij()`. Sin Detach.
- **Memoria por carpeta** en `config/local/projects/<md5 de la ruta>/`
  (bin/vkterm-state), en líneas clave=valor:

| Archivo | Lo escribe | Contenido |
|---|---|---|
| `workspace` | el plugin (`vkterm-state save`, cada segundo si cambió) | zonas visibles, foco, carpetas desplegadas y selección del explorador |
| `editor` | el plugin Lua de Micro, en cada evento si cambió | pestañas (rutas absolutas, en orden), la activa, las modificadas y las sin nombre con texto |
| `surface` | bin/vkterm | id de la pestaña de vkTERM.app, pid y app |
| `path` | bin/vkterm-state | la ruta de la carpeta |

- **Se guarda sobre la marcha** porque al cerrar la ventana no hay ocasión de
  guardar nada. El plugin Lua solo actúa en el Micro del panel Editor
  (`VKTERM_EDITOR_PANE=1`): el de `git commit` y el `$EDITOR` de los agentes
  comparten la configuración.
- **Al abrir**: bin/vkterm-editor pasa a Micro las pestañas que siguen
  existiendo (relativas al proyecto, como las nombra bin/vkterm-open) y la
  activa (`VKTERM_ACTIVE_TAB`, que aplica el plugin Lua en `postinit`); el
  plugin aplica una vez el `workspace` cuando ya están los cinco paneles y
  después la acción de `--then` (`vkterm-state load` devuelve `then=`).
  `.vkterm open=` solo se usa si la carpeta aún no tiene memoria.

### ⌘K F: cerrar la carpeta

```
Alt q → close_folder ─▶ agentes en marcha (PaneInfo: !exited && !is_held)
                    └─▶ vkterm-state close-check: modified= y unnamed= del archivo editor
   nada que perder → guarda la memoria → quit_zellij()
   si no           → ventana de confirmación (mode "confirm", src/confirm.rs)
                     Guardar y cerrar → vkterm-state save-all (vkterm.saveall en Micro)
                     Cerrar sin guardar → vkterm-state discard (vkterm.discard: ReOpen + RemoveBackup)
                     Cerrar (solo agentes) → quit_zellij()
```

Sin «Guardar y cerrar» si hay pestañas sin nombre (habría que preguntar
dónde). Cerrar la ventana del IDE no pasa por aquí: Ghostty pregunta antes
(«Close Window?», `confirm-close-surface`) y, si se acepta, Micro deja sus
copias de los cambios y ofrece recuperarlos al volver a abrir la carpeta.

### Barra de estado (mode "status")

Otra instancia del plugin en una fila al fondo (sustituye a compact-bar):
carpeta, rama de git y modo de Zellij si no es LOCKED a la izquierda; ⌘K F y
⌘K ⌘S a la derecha; el centro libre para el indicador de Ghostty. **No pide
permisos** (en un panel de una fila no se podría responder a la pregunta): el
explorador le manda la rama (`git rev-parse` cada 5 s) y el modo en un
mensaje `status`, también en cada ModeUpdate para que se redibuje con el tema
nuevo. Las ventanas flotantes reconocen al controlador como el panel fijo del
plugin que se puede enfocar (`floating::controller`).

## Plugin vkterm

Un solo plugin (`plugins/vkterm`, crate `zellij-tile` 0.45.1, target
`wasm32-wasip1`) con dos funciones, más la ventana de atajos, que es otra
instancia del mismo plugin:

| Módulo | Qué hace |
|---|---|
| `src/tree.rs` | Árbol del explorador: listados, orden (carpetas primero, sin mayúsculas), plegar / desplegar, selección estable, salto por letra, desplazamiento. Lógica pura con tests. |
| `src/layout.rs` | Genera el KDL del layout según qué zonas están visibles. Lógica pura con tests; uno comprueba que coincide con `config/zellij/layouts/vkterm.kdl`. |
| `src/main.rs` | El plugin: lee el disco (`/host`), dibuja el árbol, atiende teclas y ratón, recibe los atajos (`pipe`) y controla los paneles. |
| `src/shortcuts.rs` | Contenido y maquetación de la ventana de atajos: categorías, columnas de tres, alineación. Lógica pura con tests. |
| `src/overlay.rs` | La ventana de atajos (instancia con `mode "shortcuts"`): panel flotante, centrado y cierre. |
| `src/palette.rs` | La paleta (instancia con `mode "palette"`): ⌘P archivos y ⌘⇧P comandos en la misma ventana; lo elegido va al explorador en un mensaje `palette`, que lo ejecuta al cerrarse la ventana. |
| `src/fuzzy.rs` | Búsqueda difusa al estilo de VS Code (lógica pura, con tests). |
| `src/commands.rs` | Comandos de la paleta con su mensaje y atajo (lógica pura, con tests). |
| `src/clipboard.rs` | ⌘V en Claude Code y Codex: decide, por `clipboard info`, si el portapapeles solo tiene una imagen (lógica pura, con tests). |
| `src/memory.rs` | Memoria del workspace de cada carpeta: lectura y escritura del texto clave=valor (lógica pura, con tests). |
| `src/closing.rs` | ⌘K F: qué se perdería, el texto de la confirmación y sus botones (lógica pura, con tests). |
| `src/confirm.rs` | La confirmación de cerrar la carpeta (instancia con `mode "confirm"`). |
| `src/status.rs` | La barra de estado (instancia con `mode "status"`). |

### Explorador

- Lee las carpetas con `std::fs` bajo `/host`, que es la carpeta del proyecto.
  Solo lee una carpeta cuando se despliega.
- `watch_filesystem()` avisa de cambios en el disco y se vuelven a leer las
  carpetas afectadas.
- Al abrir un archivo comprueba si es binario (bytes NUL en los primeros 8 KB)
  y ejecuta en segundo plano `bin/vkterm-open <ruta>`
  (`run_command_with_env_variables_and_cwd`, con `ZELLIJ_SESSION_NAME`).
  Ningún panel temporal.

### Abrir un archivo en el editor (`bin/vkterm-open`)

Micro no tiene una API externa: se controla con su propia línea de comandos.

```
vkterm-open <ruta>
 ├─ zellij action list-panes      panel "Editor" (y si Micro sigue vivo)
 ├─ zellij action write(-chars)   Esc · Ctrl-E · "tabswitch 'ruta'" · Enter
 ├─ zellij action dump-screen     ¿barra de estado con la ruta o error "Could not find tab"?
 ├─ (si no estaba abierto)        Ctrl-E · "tab 'ruta'" (u "open" si el editor está vacío) · Enter
 └─ zellij action focus-pane-id   foco al editor (salvo VKTERM_NO_FOCUS=1: Espacio en el explorador)
```

También lo usa Zellij como `scrollback_editor` para `zellij edit` y para
editar el scrollback. `$EDITOR` sigue siendo `micro`.

### Controlador

Los atajos llegan como mensajes. En `config.kdl`,
`MessagePlugin "vkterm" { name "…"; }` llega al método `pipe()` del plugin,
aunque su panel esté oculto.

| Mensaje | Acción |
|---|---|
| `toggle_sidebar`, `toggle_panel`, `toggle_agents` | Oculta (`hide_pane_with_id`) o muestra (`show_pane_with_id`) la zona y vuelve a aplicar el layout |
| `focus` + `explorer`/`editor`/`claude`/`codex`/`shell` | `focus_pane_with_id` por identidad del panel; si está oculto, primero muestra su zona |
| `prev_tab`, `next_tab` | Escribe Alt-, / Alt-. en el Micro del editor y le da el foco |
| `close_tab` | Ctrl-Q a Micro, solo si el foco está en el editor |
| `edit` + `undo`/`redo`/`select_all`/`find`/`cut`/`copy` | Ctrl-Z/Y/A/F/X/C a Micro, solo si el foco está en el editor (en un shell suspenderían o interrumpirían el proceso) |
| `quick_open` | Foco al editor, Ctrl-E y `tab ` |
| `paste` | Tras el ⌘V de Ghostty: si el foco está en Claude Code o Codex y el portapapeles solo tiene una imagen (`osascript -e 'clipboard info'`, `src/clipboard.rs`), les manda ⌃V, con el que la adjuntan |
| `close_folder`, `close_folder_confirmed` | ⌘K F y la respuesta de su confirmación (ver «⌘K F: cerrar la carpeta») |
| `reload_config` | ⌘⇧,: `vkterm-theme reload` (recarga Ghostty y reactiva el modo IDE) |

**Recolocación.** Zellij no devuelve un panel oculto a su sitio al mostrarlo:
lo inserta como uno nuevo. Tampoco bastan las *swap layouts*, que eligen por
número de paneles, y ocultar el explorador da el mismo número que ocultar el
Shell. Tras cada cambio, el plugin genera el layout exacto de las zonas
visibles y lo aplica con `override_layout(…, retain=true, …)`. Zellij empareja
primero cada panel con el hueco que tiene su mismo comando o plugin. Por eso:

- el Shell va envuelto en `bin/vkterm-shell`;
- el explorador se carga siempre con el alias `vkterm`;
- `retain=true` impide que se cierre un panel aunque no encaje.

Comprobado: geometría idéntica al estado inicial tras cualquier orden de
ocultar y mostrar, también con el panel Codex convertido en shell.

**Editor siempre disponible.** Si Micro termina (⌘W o Ctrl q en la última
pestaña), el plugin relanza el panel (`rerun_command_pane`).
`bin/vkterm-editor` abre el archivo inicial solo la primera vez en cada
servidor de Zellij; después, Micro arranca vacío.

**Paneles sin foco atenuados** (como "Dim inactive split panes" de iTerm2).
Zellij no puede atenuar el contenido de un panel, pero sí cambiar su color
por defecto de texto y fondo (`set_pane_color`). En cada `PaneUpdate`, el
plugin pone a los terminales sin foco los colores `inactive_fg` /
`inactive_bg` de `config.kdl` y se los quita al que recibe el foco.

- **Alcance:** solo cambian el texto y el fondo sin color propio. El
  resaltado de sintaxis, los prompts y las barras de Micro conservan sus
  colores, igual que con `window-style` en tmux.
- **Explorador:** el plugin no puede cambiar el color por defecto de su propio
  panel. Sin foco, se dibuja con secuencias SGR en esos mismos colores.
- **Colores reales, no lo último enviado:** el plugin compara con los colores
  que Zellij informa de cada panel (`PaneInfo.default_fg/bg`). Zellij los
  reinicia al arrancar o terminar el comando de un panel y al redimensionar
  uno en espera ("Waiting to run").
- **`CSI 3 J` en cada render:** las líneas con fondo de color no se recortan
  al estrechar el panel. Al volver a mostrarlo, Zellij las parte y las pasa al
  scroll. Aunque cada render vacía el panel, el marco seguiría mostrando
  "SCROLL: 0/N"; `CSI 3 J` pone el contador a cero.

### Ventana de atajos (⌘K ⌘S)

Imita el "Keyboard Shortcuts" de Unity 7 (`unity/shortcuts`):

| | Unity | vkTERM |
|---|---|---|
| Disparo | Mantener Super (aparece a los 650 ms) | ⌘K ⌘S (Ghostty no detecta ⌘ pulsado solo) |
| Forma | Ventana centrada y semitransparente, título "Keyboard Shortcuts" | Panel flotante centrado con marco propio y título "Atajos de teclado"; el resto de paneles se atenúa |
| Contenido | Categorías en columnas de tres, atajo en negrita y descripción | Igual (`src/shortcuts.rs`); una sola columna con desplazamiento si no cabe |
| Teclas | Leídas de la configuración activa | Tabla propia; `smoke-test.py` comprueba que incluye cada atajo ⌘ del perfil de Ghostty |
| Cierre | Al soltar Super | Esc, cualquier tecla, clic, otra vez ⌘K ⌘S o al pasar el foco a otro panel |

**Cómo se abre.** Ghostty traduce la secuencia ⌘K ⌘S a Alt ?. En
`config.kdl`, Alt ? es
`MessagePlugin "vkterm" { name "toggle_shortcuts"; mode "shortcuts"; floating true; }`.
`mode` forma parte de la configuración del plugin, así que el mensaje va a otra
instancia, distinta del explorador. Si no existe, Zellij la lanza flotante. La
instancia se muestra con el primer mensaje y se cierra con el siguiente.

**Detalles comprobados en Zellij 0.45.1:**

- Un plugin lanzado por un mensaje no recibe el foco: la ventana lo pide con
  `focus_plugin_pane`.
- `is_focused` es por capa. Con la ventana a la vista, el panel fijo que tenía
  el foco lo sigue marcando. Al enfocar un panel fijo, Zellij oculta la capa
  flotante, pero la ventana sigue marcada como enfocada en ella. Por eso la
  ventana se cierra cuando su capa deja de verse (`TabUpdate`). El controlador
  no da por enfocado ningún panel fijo mientras se ve la capa flotante.
- El panel va sin marco (`borderless`) y la ventana dibuja el suyo. El de
  Zellij añade "PIN [ ]" y la ayuda para redimensionar.
- El tamaño de la zona de paneles sale solo de `PaneUpdate`. Recolocar la
  ventana genera otro `PaneUpdate`: con dos fuentes que difirieran en una fila,
  se recolocaría sin fin.
- **Permisos:** Zellij guarda por plugin los últimos permisos pedidos. La
  ventana pide los mismos cuatro que el explorador; si pidiera menos, el
  explorador volvería a preguntar en la siguiente sesión.

### Temas (⌘K ⌘T)

Como "Preferences: Color Theme" de VS Code. Cada tema de
`src/themes.rs` reúne lo que necesita cada pieza:

| Pieza | Cómo cambia en vivo | Cómo arranca con el tema guardado |
|---|---|---|
| Ghostty | `bin/vkterm-theme` escribe `config/local/theme.ghostty` y manda `SIGUSR2` al Ghostty de esta sesión (sube por los procesos padre de sus clientes de Zellij); después reactiva el modo IDE de sus pestañas | El perfil incluye `?../local/theme.ghostty` al final |
| Zellij | El selector llama a `reconfigure("theme …")` | `bin/vkterm` crea la sesión con `options --theme` |
| Micro | `bin/vkterm-theme` le manda `set colorscheme …` con `bin/vkterm-micro` | `bin/vkterm-editor` lo lanza con `-colorscheme` |
| Atenuado | El selector avisa al explorador (`pipe_message_to_plugin` a su id) | El explorador pregunta a `bin/vkterm-theme current` |

- **Selector:** es otra instancia del plugin (`mode "themes"`), como la
  ventana de atajos. ↑ ↓ (o una letra) prueban el tema; Enter lo deja; Esc, el
  atajo o perder el foco vuelven al guardado.
- **Temas propios:**
  - Light Modern de VS Code para Ghostty (`config/ghostty/themes/`).
  - Dark Modern y Light Modern para Zellij (`config/zellij/themes/`). Usan el
    formato de paleta, que Zellij trata siempre como oscuro; en el claro,
    `white` y `black` van invertidos.
- **Micro:** su esquema usa la paleta del terminal, así que sigue al tema. Los
  temas claros usan `vkterm-light.micro`: sin franja en la línea del cursor y
  barras en vídeo inverso, que se leen en cualquier paleta clara.
- **Otras ventanas** de vkTERM ya abiertas cambian al volver a abrirlas.

**Permisos.** El plugin pide ReadApplicationState, ChangeApplicationState,
WriteToStdin, RunCommands, Reconfigure (tema de Zellij) y
MessageAndLaunchOtherPlugins (avisar al explorador del tema). Zellij los
pregunta una vez en el panel del explorador y guarda la respuesta en su caché.
Todas las instancias piden los mismos: la caché guarda los últimos pedidos.

## Lenguajes (LSP)

Micro no trae LSP: lo añade el plugin [µlsp](https://github.com/Andriamanitra/mlsp)
(Lua), que instala `bin/vkterm-plugins` (Fase 2, H0).

```
Micro ─ µlsp (plug/mlsp, config/lsp/mlsp.lua) ─ stdio ─▶ lsp/bin/<servidor>
                                                          ├─ typescript-language-server (node + TypeScript 6)
                                                          ├─ pyright-langserver, ruff server
                                                          ├─ rust-analyzer (rustup)
                                                          ├─ sourcekit-lsp (xcrun)
                                                          └─ gopls (con Go local)
```

- **Versiones fijas** en `config/lsp/`: `versions` (commit de µlsp, ruff, Go y
  gopls con sha256), `package.json` y `package-lock.json` (npm), y
  `patches/`, que se aplican a µlsp con `patch(1)`.
- **Todo en el estado local:** `config/local/lsp/` y
  `config/local/micro/plug/mlsp/`. `bin/vkterm` solo copia a
  `local/micro/` la configuración de Micro, así que `plug/` persiste entre
  sesiones.
- **Rutas absolutas:** cada servidor es un ejecutable de `lsp/bin` que lanza
  el programa con su ruta (también la de node). Las sesiones arrancan con
  `zsh -l`, sin `.zshrc`, y su PATH no tiene el node de nvm.
- **`config/lsp/mlsp.lua`** sustituye a la configuración de µlsp:
  - arranca solo los servidores instalados (los busca en
    `$VKTERM_STATE_DIR/lsp/bin`);
  - activa los errores en el margen y Tab para completar;
  - pasa a rust-analyzer los `Cargo.toml` anidados;
  - formatear al guardar queda desactivado salvo `"mlsp.formatonsave": true`.
- **Raíz del proyecto** para los servidores: la carpeta de trabajo de Micro,
  que es la del proyecto.

## Teclado

En el modo IDE (tabla `vkterm` del perfil):

```
⌘ tecla ──▶ Ghostty: keybind ──▶ secuencia ──▶ Zellij (locked) ──▶ destino
⌘1..⌘5        esc:1..5             Alt 1..5      MessagePlugin focus
⌘B / ⌘J       esc:6 / esc:7        Alt 6 / 7     MessagePlugin toggle_sidebar / toggle_panel
⌥⌘B           esc:8                Alt 8         MessagePlugin toggle_agents
⌘W            esc:9                Alt 9         MessagePlugin close_tab
⌘P            esc:0                Alt 0         MessagePlugin quick_open
⌘K F          esc:q                Alt q         MessagePlugin close_folder
⌘⇧,           esc:r                Alt r         MessagePlugin reload_config
⌘⌥← → ⌃Tab    esc:, / esc:.        Alt , / .     MessagePlugin prev_tab / next_tab
⌘⌥↑ ↓         esc:k / esc:j        Alt k / j     MoveFocus
⌘S            esc:OQ               F2            Micro (Save)
⌘← / ⌘→       csi:H / csi:F        Home / End    aplicación
⌘C            acción de Ghostty    —             copiar la selección de Ghostty (+ Alt c → edit copy)
⌘V            acción de Ghostty    —             bracketed paste (+ Alt v → paste: imagen a los agentes)
```

En el modo terminal, ⌘B, ⌘J, ⌘P y ⌘⇧P envían F13–F16 con Super
(`csi:25;9~`, `26;9~`, `28;9~`, `29;9~`), que recogen los widgets de
config/zsh/vkterm.zsh; el resto de atajos son los de Ghostty.

Detalle, verificación y conflictos en [keybindings.md](keybindings.md).

## Flujo de arranque

1. **vkTERM.app** arranca el `ghostty` real con el perfil (lanzador). Cada
   pestaña abre el shell del usuario con `/usr/bin/login`, con la
   integración de zsh de Ghostty y la de vkTERM encadenadas.
2. **`vkterm .`** (bin/vkterm) decide dónde abrir el IDE (ver «`vkterm`») y,
   en el sitio, activa la tabla `vkterm` y lanza bin/vkterm-workspace como
   proceso hijo. El shell queda esperando debajo.
3. **`bin/vkterm-workspace`**:
   - Valida el directorio y las herramientas.
   - Compila el plugin si falta o hay fuentes más nuevas
     (`scripts/build-plugins.sh --if-needed`).
   - Calcula el nombre de la sesión, `vk-<carpeta>-<huella>` (6 hex de la
     ruta y de todos los archivos de `config/zellij`), recortando la carpeta para que la ruta del socket de
     Zellij no pase de 103 bytes (límite de macOS).
     `--new` añade un sufijo aleatorio.
   - Prepara el estado local, `config/local/` (o `VKTERM_STATE_DIR`): copia la
     configuración de Micro de `config/micro/` a `config/local/micro/`, donde
     Micro guarda además cursores, copias e historial, y su plugin Lua
     (`config/micro/plugins/vkterm` → `local/micro/plug/vkterm`).
   - Copia `config/zellij` a `config/local/zellij/<huella>/` si no existe, y
     borra las copias de sesiones que ya no existen. La sesión usa siempre
     esa copia (ver "Copia de la configuración por sesión").
   - Elige el archivo inicial (argumento o, sin memoria, `open=` en `.vkterm`).
   - Exporta `VKTERM_HOME`, que `config.kdl` usa para localizar el `.wasm`
     (Zellij expande variables en la ruta del plugin). Exporta también
     `VKTERM_STATE_DIR`, `VKTERM_PROJECT`, `VKTERM_PROJECT_STATE`,
     `VKTERM_OPEN_FILE`, `VKTERM_THEN`, `EDITOR` / `VISUAL=micro` y
     `MICRO_CONFIG_HOME`, y añade `bin/` al PATH.
   - Si queda una sesión viva con ese nombre, se adjunta a ella. Si no, la
     crea en el proyecto con el tema de Zellij guardado (`options --theme`).
4. Zellij carga `config.kdl` de esa copia:
   - modo locked, tema y atajos;
   - alias `plugins { vkterm … }`;
   - `scrollback_editor "vkterm-open"`;
   - `on_force_close "quit"`.

   Después carga `layouts/vkterm.kdl`, y el plugin aplica la memoria de la
   carpeta.

**Copia de la configuración por sesión.** Zellij recarga `config.kdl` en
caliente, pero el plugin de una sesión sigue siendo el de la versión con la que
arrancó. Con la carpeta del repositorio, al actualizar vkTERM las sesiones
abiertas recibían atajos que su plugin no conocía: ⌘P abrió con el plugin
antiguo un segundo explorador flotante. Con la copia, cada sesión conserva la
configuración con la que se creó, y una configuración nueva crea otra sesión.
Además, el plugin se cierra si le llega un `mode` que no conoce.

No se copia el `.wasm`: Zellij guarda los permisos concedidos por ruta del
plugin y volvería a preguntarlos en cada sesión.

## Aislamiento respecto a la configuración del usuario

| Componente | Mecanismo | Qué no se toca |
|---|---|---|
| Ghostty | vkTERM.app (otro identificador) con `--config-default-files=false` + `--config-file` | `~/.config/ghostty` y la Ghostty.app del usuario |
| Zellij | `--config-dir config/local/zellij/<huella>` (copia de `config/zellij`) | `~/.config/zellij` |
| Micro | `MICRO_CONFIG_HOME=config/local/micro` (copia de `config/micro`) | `~/.config/micro` |
| Shell | ZDOTDIR apunta a `config/zsh`, que carga los archivos del usuario y devuelve ZDOTDIR a su valor; no se modifica ningún rc | `~/.zshrc`, `~/.zprofile`, `~/.zshenv` |

Estado que sí se escribe fuera del repositorio, porque las herramientas no
permiten moverlo:

- Zellij: caché de plugins compilados y **permisos concedidos al plugin**
  (`~/Library/Caches/org.Zellij-Contributors.Zellij/`), socket y logs
  (`$TMPDIR/zellij-<uid>/`).
- `bin/vkterm-editor`: una marca vacía por sesión en `$TMPDIR`.
- `config/zsh/vkterm.zsh`: una carpeta vacía por ventana abierta ya convertida
  (`$TMPDIR/vkterm-autostart-<nonce>`).
- `bin/vkterm-app`: `~/Applications/vkTERM.app` (y `vkTERM Test.app` en
  `~/Library/Caches/vkterm`), su registro en LaunchServices, el enlace
  `~/.local/bin/vkterm` con `--link` y los permisos que macOS guarda para la
  app (carpetas, Automatización).
- Cargo: caché de crates en `~/.cargo/registry` (descargadas de crates.io).
- Dentro del repositorio, ignorados por git: `plugins/vkterm/target/` y el
  estado local, `config/local/` (tema elegido, datos de Micro y copias de la
  configuración de Zellij por sesión). Las versiones anteriores guardaban el
  estado de Micro en `config/micro/buffers/` y `backups/`, también ignorados.

## Decisiones de diseño

| Decisión | Motivo |
|---|---|
| Plugin propio en lugar de Strider | Strider abría terminales con Enter sin selección, no se puede ocultar y no muestra un árbol. Requisito: comportarse como VS Code. |
| Un solo plugin (explorador + controlador) | Recibe los atajos aunque esté oculto; evita mensajes entre plugins. |
| `override_layout` tras ocultar / mostrar | Única forma comprobada de devolver cada panel a su hueco exacto. |
| Ocultar en lugar de cerrar | Decisión del usuario: la conversación de Claude Code y los procesos no se pierden. |
| Layout VS Code (Shell abajo, agentes a la derecha) | Decisión del usuario. Barra derecha = barra secundaria de VS Code, donde va Claude Code. |
| ⌘⌥← → para pestañas | Decisión del usuario (como VS Code). El foco entre paneles pasa a ⌘1–⌘5 por identidad. |
| Abrir archivos con `bin/vkterm-open` | Protocolo ya probado (corrección 2); el plugin lo ejecuta en segundo plano. |
| Ventana de atajos con ⌘K ⌘S | Petición del usuario (como Unity). Mantener ⌘ pulsado exigiría un programa de macOS con permiso de monitorización de entrada; ⌘K ⌘S es el atajo de VS Code y solo usa letras (teclado Spanish - ISO). |
| Temas que cambian Ghostty, Zellij y Micro a la vez | Como en VS Code, el tema afecta a toda la ventana. Estado en `config/local/` (dentro del proyecto, fuera de git). |
| Atenuar los paneles sin foco con `set_pane_color` | Petición del usuario (como iTerm2). Zellij 0.45.1 no tiene opción propia; es la técnica de `window-style` de tmux. |
| Ruta del plugin con `$VKTERM_HOME` | Zellij expande variables en `file:`; evita generar la configuración. |
| Modo **locked** por defecto | Micro, zsh y los agentes usan Ctrl s/q/o/t/p/n/b. |
| ⌘S → F2 | Ctrl-S congela la salida en los shells (XON/XOFF). |
| ⌘C / ⌘V nativos de Ghostty | ⌘C nunca debe mandar Ctrl-C (SIGINT). |
| Los dos ⌥ escriben caracteres | Como en VS Code y cualquier app de macOS: ⌥2 = @ con cualquiera de los dos (el usuario lo hace con el derecho, como AltGr en Windows). Todo lo de vkTERM se alcanza con ⌘. |
| Agentes con `start_suspended` | Paneles visibles y preparados, sin lanzar agentes sin consentimiento. |
| Hash de configuración en el nombre de la sesión | Evita reanudar una sesión con un layout antiguo sin destruirla. |
| Copia de la configuración de Zellij por sesión | Zellij recarga `config.kdl` en caliente, pero el plugin de una sesión es el de su versión: atajos nuevos con un plugin viejo abrieron un segundo explorador. |
| Sin forks de Zellij ni de Ghostty | La API de plugins de Zellij 0.45.1 ha bastado. |
| Modo terminal y modo IDE por pestaña | Decisión del usuario (22-10-2026): vkTERM es el terminal de todos los días y `vkterm .` convierte la pestaña, como `code .`. |
| vkTERM.app = copia local de Ghostty.app | Icono y nombre propios en el Dock, y un identificador propio para AppleScript sin chocar con el Ghostty del usuario. Sin fork: el mismo binario. |
| Tablas de atajos de Ghostty + AppleScript | Única forma de que cada pestaña tenga sus atajos: un programa no puede cambiar los atajos de Ghostty de otra manera. |
| El IDE como proceso hijo del shell | El shell de antes queda intacto debajo (scroll, historial); ⌘K F vuelve a él. |
| La sesión dura lo que su pestaña, con memoria | Decisión del usuario: al cerrar la carpeta o la ventana se cierra todo, pero la carpeta vuelve como estaba. Las conversaciones de los agentes no se recuerdan. |
| ⌘K F pregunta solo si se perdería algo | Decisión del usuario, como VS Code al cerrar con cambios. |
| Barra de estado propia | El indicador de la tabla de Ghostty no se puede ocultar: así queda sobre una barra y no sobre el Shell (decisión del usuario). |

## Estructura del repositorio

```
vkTERM/
├── bin/
│   ├── vkterm                 El comando `vkterm`: convierte la pestaña, abre una ventana o enfoca
│   ├── vkterm-workspace       El IDE en el terminal actual: plugin, estado y sesión de Zellij
│   ├── vkterm-app             Crea vkTERM.app (copia local de Ghostty.app)
│   ├── vkterm-surface         Modo de cada pestaña de vkTERM.app (AppleScript)
│   ├── vkterm-state           Memoria de cada carpeta
│   ├── vkterm-editor          Proceso del panel Editor (Micro + pestañas recordadas)
│   ├── vkterm-shell           Proceso del panel Shell ($SHELL)
│   ├── vkterm-open            Abre un archivo en una pestaña del Micro del editor
│   ├── vkterm-micro           Ejecuta un comando en el Micro del editor
│   ├── vkterm-plugins         Instala el LSP (µlsp y servidores) con versiones fijas
│   ├── vkterm-theme           Guarda el tema (⌘K ⌘T) y lo aplica a Ghostty y Micro
│   └── vkterm-ghostty         Nombre anterior de `vkterm` (enlaces antiguos)
├── config/
│   ├── zellij/
│   │   ├── config.kdl         Modo, tema, alias del plugin, atajos → mensajes al plugin
│   │   ├── layouts/vkterm.kdl Explorador | editor / Shell | Claude Code / Codex
│   │   └── themes/vkterm.kdl  Dark Modern y Light Modern para Zellij
│   ├── micro/                 settings, bindings, colores y plugins/vkterm (se copian a local/micro)
│   ├── zsh/                   Integración de zsh de vkTERM.app (modo terminal)
│   ├── lsp/                   Versiones fijas, parches y configuración de µlsp
│   ├── ghostty/
│   │   ├── vkterm.ghostty     Perfil de Ghostty: modo terminal y tabla `vkterm` (modo IDE)
│   │   └── themes/            Light Modern para Ghostty
│   └── local/                 Estado (fuera de git): tema elegido, datos de Micro
├── plugins/vkterm/            Plugin de Zellij (Rust → wasm32-wasip1)
├── scripts/
│   ├── build-plugins.sh       Compila el plugin (--if-needed, --test)
│   ├── vkterm-app-launcher.c  Ejecutable principal de vkTERM.app
│   ├── doctor.sh              Auditoría del entorno (solo lectura)
│   ├── smoke-test.py          Pruebas: configuración + PTY (+ Ghostty con --ghostty)
│   └── send-keys.swift        Teclas sintetizadas para la prueba de Ghostty
├── examples/demo/             Proyecto mínimo; .vkterm abre Header.tsx
├── docs/
│   ├── architecture.md        Este documento
│   ├── keybindings.md         Atajos, cómo se verificaron y conflictos
│   ├── manual-tests.md        Lista para probar con el teclado físico
│   ├── phase-0-report.md      Fase 0 y correcciones 1 y 2
│   ├── phase-1-plan.md        Plan aprobado de la Fase 1
│   ├── phase-1-report.md      Entrega de la Fase 1
│   ├── phase-2-plan.md        Plan aprobado de la Fase 2
│   ├── phase-2-report.md      Entrega de la Fase 2 (por hitos)
│   └── next-steps.md          Próximos pasos: GitHub y plugins
├── LICENSE                    GPL-3.0-or-later (decidido el 22-10-2026)
└── THIRD_PARTY_NOTICES.md     Licencias de los componentes y crates usados
```
