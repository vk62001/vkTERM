# Arquitectura de vkTERM — Fase 1 (interfaz tipo VS Code)

vkTERM combina componentes estables sin forks: Ghostty, Zellij y Micro. A eso
añade un plugin propio de Zellij, `vkterm` (Rust → WebAssembly). El plugin
hace de explorador y de controlador del workspace.

## Vista general

```
Ghostty (perfil config/ghostty/vkterm.ghostty: capa ⌘, ⌥ = caracteres)
 └─ /usr/bin/login → zsh -l                       PATH completo del usuario
     └─ bin/vkterm [--new] <proyecto> [archivo]    compila el plugin si hace falta; sesión por proyecto
         └─ zellij client ⇄ zellij server          --config-dir config/zellij
             ├─ Explorador 20%     plugin vkterm (plugins/vkterm): árbol + controlador
             ├─ Editor             bin/vkterm-editor → micro (pestañas)
             ├─ Shell (abajo 30%)  bin/vkterm-shell → $SHELL
             └─ Barra derecha 25%
                 ├─ Claude Code    claude  (start_suspended: espera Enter)
                 └─ Codex          codex   (start_suspended: espera Enter)
```

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
| Ghostty | `bin/vkterm-theme` escribe `config/local/theme.ghostty` y manda `SIGUSR2` al Ghostty de esta sesión (sube por los procesos padre de sus clientes de Zellij) | El perfil incluye `?../local/theme.ghostty` al final |
| Zellij | El selector llama a `reconfigure("theme …")` | `bin/vkterm` crea la sesión con `options --theme` |
| Micro | `bin/vkterm-theme` le manda `set colorscheme …` como `vkterm-open` | `bin/vkterm-editor` lo lanza con `-colorscheme` |
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

## Teclado

```
⌘ tecla ──▶ Ghostty: keybind ──▶ secuencia ──▶ Zellij (locked) ──▶ destino
⌘1..⌘5        esc:1..5             Alt 1..5      MessagePlugin focus
⌘B / ⌘J       esc:6 / esc:7        Alt 6 / 7     MessagePlugin toggle_sidebar / toggle_panel
⌥⌘B           esc:8                Alt 8         MessagePlugin toggle_agents
⌘W            esc:9                Alt 9         MessagePlugin close_tab
⌘P            esc:0                Alt 0         MessagePlugin quick_open
⌘⌥← → ⌃Tab    esc:, / esc:.        Alt , / .     MessagePlugin prev_tab / next_tab
⌘⌥↑ ↓         esc:k / esc:j        Alt k / j     MoveFocus
⌘S            esc:OQ               F2            Micro (Save)
⌘← / ⌘→       csi:H / csi:F        Home / End    aplicación
⌘C / ⌘V       acciones de Ghostty  —             copiar selección de Ghostty / bracketed paste
```

Detalle, verificación y conflictos en [keybindings.md](keybindings.md).

## Flujo de arranque

1. **`bin/vkterm-ghostty [--new] [DIR] [ARCHIVO]`** lanza una instancia nueva
   de Ghostty con `open -na`, el perfil del proyecto y
   `--config-default-files=false`. Los argumentos viajan como variables de
   entorno: si fueran argumentos, AppKit los trataría como archivos a abrir.
2. Ghostty ejecuta `bin/vkterm` a través de `/bin/zsh -lc`, porque arranca los
   comandos con el PATH mínimo de launchd.
3. **`bin/vkterm`**:
   - Valida el directorio y las herramientas.
   - Compila el plugin si falta o hay fuentes más nuevas
     (`scripts/build-plugins.sh --if-needed`).
   - Elige el archivo inicial (argumento o `open=` en `.vkterm`).
   - Calcula el nombre de la sesión, `vk-<carpeta>-<hash ruta>-<hash config>`.
     `--new` añade la hora.
   - Prepara el estado local, `config/local/` (o `VKTERM_STATE_DIR`): copia la
     configuración de Micro de `config/micro/` a `config/local/micro/`, donde
     Micro guarda además cursores, copias e historial.
   - Exporta `VKTERM_HOME`, que `config.kdl` usa para localizar el `.wasm`
     (Zellij expande variables en la ruta del plugin). Exporta también
     `VKTERM_STATE_DIR`, `VKTERM_PROJECT`, `VKTERM_OPEN_FILE`,
     `EDITOR` / `VISUAL=micro` y `MICRO_CONFIG_HOME`, y añade `bin/` al PATH.
   - Si la sesión existe, se adjunta a ella. Si no, la crea en el proyecto con
     el tema de Zellij guardado (`options --theme`).
4. Zellij carga `config/zellij/config.kdl`:
   - modo locked, tema y atajos;
   - alias `plugins { vkterm … }`;
   - `scrollback_editor "vkterm-open"`.

   Después carga `config/zellij/layouts/vkterm.kdl`.

## Aislamiento respecto a la configuración del usuario

| Componente | Mecanismo | Qué no se toca |
|---|---|---|
| Ghostty | `--config-default-files=false` + `--config-file` | `~/.config/ghostty` |
| Zellij | `--config-dir config/zellij` | `~/.config/zellij` |
| Micro | `MICRO_CONFIG_HOME=config/local/micro` (copia de `config/micro`) | `~/.config/micro` |
| Shell | Solo hereda variables; no se modifica ningún rc | `~/.zshrc`, `~/.zprofile` |

Estado que sí se escribe fuera del repositorio, porque las herramientas no
permiten moverlo:

- Zellij: caché de plugins compilados y **permisos concedidos al plugin**
  (`~/Library/Caches/org.Zellij-Contributors.Zellij/`), socket y logs
  (`$TMPDIR/zellij-<uid>/`).
- `bin/vkterm-editor`: una marca vacía por sesión en `$TMPDIR`.
- Cargo: caché de crates en `~/.cargo/registry` (descargadas de crates.io).
- Dentro del repositorio, ignorados por git: `plugins/vkterm/target/` y el
  estado local, `config/local/` (tema elegido y datos de Micro). Las versiones
  anteriores guardaban el estado de Micro en `config/micro/buffers/` y
  `backups/`, también ignorados.

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
| Sin forks de Zellij ni de Ghostty | La API de plugins de Zellij 0.45.1 ha bastado. |

## Estructura del repositorio

```
vkTERM/
├── bin/
│   ├── vkterm                 Lanzador: compila el plugin si hace falta; sesión Zellij por proyecto
│   ├── vkterm-ghostty         Abre vkterm en una ventana de Ghostty con el perfil propio
│   ├── vkterm-editor          Proceso del panel Editor (Micro + archivo inicial)
│   ├── vkterm-shell           Proceso del panel Shell ($SHELL)
│   ├── vkterm-open            Abre un archivo en una pestaña del Micro del editor
│   └── vkterm-theme           Guarda el tema (⌘K ⌘T) y lo aplica a Ghostty y Micro
├── config/
│   ├── zellij/
│   │   ├── config.kdl         Modo, tema, alias del plugin, atajos → mensajes al plugin
│   │   ├── layouts/vkterm.kdl Explorador | editor / Shell | Claude Code / Codex
│   │   └── themes/vkterm.kdl  Dark Modern y Light Modern para Zellij
│   ├── micro/                 settings, bindings, colores (se copian a local/micro)
│   ├── ghostty/
│   │   ├── vkterm.ghostty     Perfil de Ghostty: capa ⌘, ⌥ = caracteres
│   │   └── themes/            Light Modern para Ghostty
│   └── local/                 Estado (fuera de git): tema elegido, datos de Micro
├── plugins/vkterm/            Plugin de Zellij (Rust → wasm32-wasip1)
├── scripts/
│   ├── build-plugins.sh       Compila el plugin (--if-needed, --test)
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
│   └── next-steps.md          Próximos pasos: GitHub y plugins
├── LICENSE                    GPL-3.0-or-later (decidido el 22-10-2026)
└── THIRD_PARTY_NOTICES.md     Licencias de los componentes y crates usados
```
