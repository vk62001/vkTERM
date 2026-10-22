# vkTERM — Fase 1 redefinida: interfaz tipo VS Code

## Contexto

La Fase 0 se montó con piezas existentes: Ghostty, Zellij, Strider y Micro.
En la revisión con el teclado real, el explorador **Strider** sigue abriendo
terminales: Enter sin nada seleccionado abre uno en esa carpeta, y al entrar en
una carpeta nunca queda nada seleccionado. Esto no se puede desactivar desde la
configuración. Tampoco se puede ocultar ni mostrar Strider, ni enseñar el
proyecto como árbol.

El usuario redefine el objetivo: **vkTERM debe comportarse como VS Code**. La
prioridad inmediata es la barra lateral:

- ⌘B la muestra y la oculta.
- Moverse entre archivos.
- Abrir varios archivos a la vez.

Después, poder abrir y cerrar los terminales de Claude Code, Codex y Shell
cuando se quiera.

**Decisiones del usuario (2026-10-21):**

- **Layout como VS Code:**
  - Explorador a la izquierda (⌘B).
  - Editor en el centro, con pestañas.
  - Shell en un panel inferior bajo el editor (⌘J).
  - Claude Code y Codex en una barra lateral derecha (⌥⌘B).
- **"Cerrar" un terminal = ocultarlo**: el proceso sigue vivo y la conversación
  con Claude no se pierde.
- **⌘⌥← / ⌘⌥→** cambian de pestaña en el editor, como en VS Code. El foco entre
  paneles pasa a ⌘1–⌘5, ahora por identidad del panel.

```
┌──────────┬────────────────────────┬────────────┐
│ EXPLORER │ Header.tsx  [App.tsx]  │ Claude Code│
│ ▾ src    │                        │            │
│   ▸ comp │  (Micro, pestañas)     ├────────────┤
│   App.tsx│                        │ Codex      │
│          ├────────────────────────┤            │
│   ⌘B     │ Shell            ⌘J    │      ⌥⌘B   │
└──────────┴────────────────────────┴────────────┘
```

## Enfoque

Se crea un **plugin propio de Zellij, `vkterm`** (Rust → `wasm32-wasip1`, crate
`zellij-tile` 0.45.1, sin forks). Sustituye a Strider y hace dos funciones:

1. **Explorador en árbol** al estilo VS Code. Nunca abre terminales.
2. **Controlador del workspace**, que recibe los atajos y:
   - oculta y muestra paneles (`hide_pane_with_id` / `show_pane_with_id`);
   - enfoca por identidad (`focus_pane_with_id`);
   - vuelve a colocar cada panel en su sitio (`override_layout`).

**Se mantiene:**

- Ghostty con la capa ⌘.
- Zellij para paneles y sesiones.
- Micro con pestañas.
- `bin/vkterm-open`, el protocolo `tabswitch`/`tab` ya probado.

### Hechos verificados en el código fuente de Zellij v0.45.1

- **Ocultar:** existen `hide_pane_with_id(PaneId)` y `hide_self()`. Al ocultar
  un panel, los demás ocupan su espacio y el proceso sigue vivo.
- **Mostrar:** existe `show_pane_with_id(PaneId, float, focus)`, pero **no
  devuelve el panel a su sitio**: lo inserta como uno nuevo. Las *swap layouts*
  eligen por número de paneles, y ocultar el explorador da el mismo número que
  ocultar el Shell, así que no bastan.
- **Recolocar:** existe `override_layout(LayoutInfo, retain_terminals,
  retain_plugins, solo_pestaña_activa, ctx)`. Asigna cada panel a su hueco
  **primero por coincidencia exacta** de comando o plugin (`run`). El
  controlador genera el layout exacto del estado visible y lo aplica tras cada
  ocultar o mostrar, con `retain=true` para no matar nunca procesos.
- **Atajos → plugin:** el atajo KDL `MessagePlugin "<url|alias>" { name "toggle_sidebar" }`
  llega al método `pipe()` del plugin con `PipeSource::Keybind`. Funciona aunque
  el plugin esté oculto.
- **Abrir archivos y pestañas:** `run_command_with_env_variables_and_cwd`
  ejecuta `vkterm-open` en segundo plano. Desaparece el panel temporal de la
  corrección 2. `write_to_pane_id` envía a Micro el cambio de pestaña.
- **Interfaz del explorador:**
  - Lee el disco con `std::fs` bajo `/host`, que es la carpeta del proyecto.
  - Se actualiza con `watch_filesystem()`, con 400 ms de debounce.
  - Recibe teclas y ratón: `LeftClick`, `ScrollUp` / `ScrollDown`.
  - Dibuja con `NestedListItem::indent().selected()`.
- **Permisos:** ReadApplicationState, ChangeApplicationState, WriteToStdin y
  RunCommands. Zellij los pide **una vez** al cargar el plugin y guarda la
  respuesta en su caché.

### Atajos (Ghostty → código interno → Zellij → mensaje al plugin)

Zellij solo ve Alt+tecla. Ghostty traduce cada ⌘ a un código Alt libre. Todo
llega a Zellij aunque esté en modo LOCKED.

| Atajo | Código | Acción |
|---|---|---|
| ⌘B | Alt 6 | Mostrar u ocultar el explorador. El foco no cambia; si estaba en el explorador, pasa al editor. |
| ⌘J | Alt 7 | Mostrar u ocultar el Shell. Al mostrarlo recibe el foco, como en VS Code. |
| ⌥⌘B | Alt 8 | Mostrar u ocultar Claude Code y Codex. |
| ⌘1 … ⌘5 | Alt 1–5 | Foco por identidad en explorador, editor, Claude, Codex o Shell. Si el panel está oculto, lo muestra. |
| ⌘⌥← / ⌘⌥→, ⌃⇧Tab / ⌃Tab | Alt , / Alt . | Pestaña anterior o siguiente de Micro, con el foco en el editor. |
| ⌘W | Alt 9 | En el editor cierra la pestaña (Ctrl-Q de Micro). En otro panel no hace nada. Hoy ⌘W cierra la ventana de Ghostty. |
| ⌘P | Alt 0 | Sin cambios por ahora (prompt `tab` de Micro). |

Se sobrescriben valores por defecto de Ghostty:

- ⌘J: scroll a la selección.
- ⌘W: cerrar ventana.
- ⌃Tab y ⌃⇧Tab: pestañas de Ghostty.

### Explorador (comportamiento VS Code)

- **Contenido:**
  - Cabecera `EXPLORER: <PROYECTO>`.
  - Árbol con ▸/▾, carpetas primero y orden sin mayúsculas, como VS Code.
  - Raíz fija en el proyecto; no se puede subir más arriba.
- **Teclado:**
  - ↑ ↓: mover.
  - →: expandir la carpeta, o bajar a su primer hijo.
  - ←: plegar, o subir al padre.
  - Enter: en un archivo, lo abre en una pestaña y pasa el foco al editor; en
    una carpeta, la pliega o despliega.
  - Espacio: abre el archivo sin quitar el foco del explorador.
  - Inicio / Fin y Re Pág / Av Pág.
  - Escribir una letra salta a la siguiente entrada que empieza por ella.
- **Ratón:** clic selecciona; clic en archivo lo abre; clic en carpeta la
  pliega o despliega; la rueda desplaza.
- **Archivos excluidos** (`files.exclude` por defecto de VS Code): `.git` y
  `.DS_Store`. Ctrl-H alterna ver los ocultos.
- **Binarios** (NUL en los primeros 8 KB): no se abren; aviso en el pie del
  explorador.
- El árbol se refresca solo cuando cambian los archivos. Al ocultar y volver a
  mostrar, conserva las carpetas abiertas y la selección.

## Hitos (cada uno se entrega para revisión)

### H0 — Prueba técnica (sí/no) y herramientas

- **Instalar, con tu aprobación** (son descargas oficiales):
  - `rustup target add wasm32-wasip1`;
  - `zellij-tile = "0.45.1"` desde crates.io.
- Plugin mínimo cargado desde el layout. Hay que comprobar:
  1. Ocultar el explorador → el editor se ensancha. Mostrarlo → vuelve a la
     izquierda con el mismo ancho, aunque se hayan ocultado otros paneles en
     cualquier orden.
  2. Las PIDs del Shell, Micro y de los terminales en espera no cambian.
  3. `MessagePlugin` desde un atajo funciona en LOCKED y con el plugin oculto,
     usando un alias de `plugins {}`.
  4. `run_command` de `vkterm-open` con `ZELLIJ_SESSION_NAME` funciona.
  5. Ghostty → Alt 6–9 / , / . llega a Zellij sin afectar a Micro ni a zsh.
  6. Cómo se resuelve la ruta `file:` del `.wasm`.
  7. Cómo responder al aviso de permisos en las pruebas automáticas.
- **Si (1) falla**, se comunica antes de seguir. El plan B es explorador
  flotante (sin redistribuir el espacio), menos parecido a VS Code.

### H1 — Barra lateral (prioridad)

- Nuevo layout VS Code, con cada panel identificado de forma única para
  `override_layout`.
- Explorador en árbol completo.
- ⌘B.
- Abrir varios archivos en pestañas.
- ⌘⌥← → y ⌃Tab.
- ⌘1 por identidad.
- ⌘W seguro.
- Se elimina Strider.

### H2 — Terminales

- ⌘J y ⌥⌘B.
- ⌘2–⌘5 por identidad, mostrando el panel si estaba oculto (sustituyen a las
  macros posicionales).
- Al ocultar un panel enfocado, el foco pasa al editor.

### H3 — Después, no en esta fase

- ⌘P con búsqueda difusa.
- Conservar los tamaños que el usuario haya redimensionado.
- ⌘W en la última pestaña: dejar el editor vacío en vez de cerrar Micro.
- Crear, renombrar y borrar desde el explorador.
- Colores de git.

## Archivos

**Nuevos:**

- `plugins/vkterm/`:
  - `Cargo.toml`, con `zellij-tile = "0.45.1"`.
  - `.cargo/config.toml`, con `target = "wasm32-wasip1"`.
  - `src/main.rs`: `ZellijPlugin`, con `load` (permisos y suscripciones),
    `update`, `pipe` (mensajes de los atajos) y `render`.
  - `src/tree.rs`: modelo del árbol, lógica pura con tests.
  - `src/layout.rs`: genera el KDL del estado visible, lógica pura con tests.
  - `src/panes.rs`: identifica los paneles a partir de `PaneManifest`.
  - `src/render.rs`.
- `scripts/build-plugins.sh`.

**Modificados:**

- `config/zellij/layouts/vkterm.kdl`: nuevo layout; plugin `vkterm` en lugar de
  Strider.
- `config/zellij/config.kdl`:
  - alias `plugins { vkterm … }`;
  - atajos `MessagePlugin`;
  - se quitan las macros posicionales y Strider.
- `config/ghostty/vkterm.ghostty`: ⌘B, ⌘J, ⌥⌘B, ⌘W, ⌘⌥← →, ⌃Tab.
- `bin/vkterm`:
  - compila el plugin si falta o está desactualizado;
  - si H0 confirma que hace falta, genera la config con la ruta absoluta del
    `.wasm` en `$TMPDIR`.
- `bin/vkterm-open`: acepta la sesión desde el entorno y se elimina el bloque
  que corregía Strider.
- `scripts/smoke-test.py`, `scripts/doctor.sh` (el target wasm pasa a ser
  obligatorio).
- `docs/`, `README.md`, `plugins/README.md`.

## Verificación

- **`cargo test`** (host): árbol, orden, exclusiones, navegación ← → ↑ ↓, salto
  por letra y generación de layouts.
- **`scripts/smoke-test.py`** (PTY) y **`--ghostty`** (teclas sintetizadas):
  - ⌘B oculta el explorador (`is_suppressed`), el editor se ensancha, y al
    volver recupera la misma geometría (±1). Se repite combinado con ⌘J y ⌥⌘B
    en distintos órdenes.
  - Las PIDs no cambian tras ocultar y mostrar.
  - Explorador: navegar, expandir y plegar, abrir 3 archivos → 3 pestañas en
    Micro y el número de paneles no cambia. Enter en una carpeta nunca abre un
    terminal. Raíz fija.
  - ⌘1–⌘5 enfocan aunque el panel esté oculto o se haya movido.
  - ⌘⌥← → y ⌃Tab cambian de pestaña.
  - ⌘W cierra la pestaña y no la ventana.
  - Se mantiene todo lo de la corrección 1 (⌘S, ⌘C, ⌘V, LOCKED, sesiones),
    adaptado al nuevo layout.
- **Capturas** de la ventana real de Ghostty (`screencapture -l`) en cada
  estado. Ya está probado que funciona.
- **Lista manual actualizada** (`docs/manual-tests.md`) para el teclado
  físico.

## Reglas que se mantienen

- Sin sudo ni forks.
- Nada de configuración global (la caché de permisos de Zellij es suya).
- **Sin commits salvo que lo pidas.** Recomiendo un commit base de la Fase 0
  antes de empezar H1, solo con tu OK.
- Este plan se copiará a `docs/phase-1-plan.md` para la revisión.
