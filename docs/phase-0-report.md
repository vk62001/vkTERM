# Informe de la Fase 0 — Base funcional

Fecha: 2026-10-20 (corrección 2: 2026-10-21) · Equipo: Mac mini Apple M4 ·
Teclado: Spanish - ISO
Estado: **cerrada.** La revisión de la corrección 2 redefinió el proyecto
(interfaz tipo VS Code): ver [phase-1-plan.md](phase-1-plan.md) y
[phase-1-report.md](phase-1-report.md).

## 1. Correcciones

### Corrección 2 (explorador → editor central)

| Problema de la revisión | Causa | Corrección | Evidencia |
|---|---|---|---|
| Los archivos abiertos desde el explorador aparecían en paneles nuevos junto al explorador, no en el editor central | Strider llama a `open_file()`. Zellij abre entonces un panel nuevo con el editor junto al panel enfocado (el explorador) y, si falta espacio, lo apila sobre él. | Zellij permite cambiar el programa (`scrollback_editor`), no el sitio. vkTERM usa `bin/vkterm-open`: abre el archivo como **pestaña del Micro central** (o salta a su pestaña si ya está abierto), le da el foco y termina; Zellij cierra solo el panel temporal (≈0,3 s). Sin plugins: usa los comandos `tab` / `tabswitch` de Micro. | PTY y Ghostty: pestaña nueva, foco en el editor, siguen 5 paneles y un solo Micro; reabrir no duplica pestañas. También con nombres con espacios y apóstrofos, rutas largas fuera del proyecto, un proyecto sin archivo inicial (sustituye el búfer vacío) y Micro cerrado (lo vuelve a abrir en el panel Editor). |
| "En el lado de archivos me abre terminales" | Strider abre un **terminal** cuando se pulsa Enter (o → / Tab) sin nada seleccionado. La selección queda vacía al entrar en una carpeta y tras abrir un archivo. | Tras abrir un archivo, `vkterm-open` devuelve Strider a la carpeta con la primera entrada seleccionada. Al entrar en una carpeta sigue pasando: **pendiente** (requiere plugin propio). | Prueba: tras abrir un archivo, Enter entra en `components/` sin abrir terminal. |
| Un PNG se abrió en Micro como texto | Strider abre cualquier archivo con el editor. | Los binarios no se abren: aviso de 5 s junto al explorador. | Prueba con un PNG. |
| ⌘P sustituía el archivo actual | Ctrl-O (`open`) reutiliza la pestaña activa. | ⌘P prepara `tab ` en la línea de comandos de Micro: pestaña nueva. | Prueba PTY y Ghostty. |

`$EDITOR` sigue siendo `micro`: `scrollback_editor` solo lo usa Zellij, así que
git y los agentes no pasan por `vkterm-open` (comprobado).

### Corrección 1 (revisión en Ghostty)

| Problema de la revisión | Causa | Corrección | Evidencia |
|---|---|---|---|
| Los atajos no funcionaban con el teclado físico | Solo existía la capa Alt con el ⌥ **izquierdo** (`option-as-alt = left`) y no había atajos ⌘. Mi validación anterior inyectaba bytes en un PTY y se saltaba Ghostty. | Capa ⌘ en el perfil de Ghostty (traducida a secuencias que entienden Zellij y Micro), `macos-option-as-alt = right` y validación dentro de Ghostty real. | Ghostty efectivo (`+show-config`, `+list-keybinds`). Prueba `--ghostty` con eventos sintetizados: ⌘1–5, ⌘⌥ flechas, ⌘S, ⌘P, ⌘V, ⌘C, ⌘← →, ⌥ derecho y ⌥ izquierdo. **Pulsaciones físicas: pendientes** (`docs/manual-tests.md`). |
| Terminales apiladas, una sola visible | `stacked=true` en el layout. | Columna derecha dividida en tres paneles. | 3 paneles visibles, ninguno suprimido, sin `stacked` en `dump-layout`. Filas 17/16/16 a 200×50 y 19/18/18 en Ghostty a 239×59. |
| Micro vacío ("No name") en el demo | El panel ejecutaba `micro` sin argumentos. | `examples/demo/.vkterm` (`open=src/components/Header.tsx`) + `bin/vkterm-editor`. | La barra de Micro muestra `src/components/Header.tsx`, también tras reiniciar la sesión. |
| Explorador no integrado con el editor | Strider siempre abre los archivos en un panel nuevo. | Pendiente en la corrección 1; resuelto en la corrección 2 sin plugin. | Ver corrección 2. |
| Combinaciones poco naturales para macOS | Navegación basada en Alt y Ctrl g. | ⌘1–5, ⌘⌥ flechas, ⌘S, ⌘P, ⌘← →; ⌘C / ⌘V nativos; ⌥ izquierdo para caracteres. | `docs/keybindings.md`. |
| (Riesgo detectado) `bin/vkterm` se adjuntaba a sesiones con un layout antiguo | El nombre de la sesión dependía solo del proyecto. | Hash de configuración en el nombre de la sesión y opción `--new`. | La sesión `vk-demo-e84d` de tu revisión quedó intacta durante todas las pruebas. |

## 2. Auditoría del entorno

Reproducible con `scripts/doctor.sh`.

| Herramienta | Versión | Estado |
|---|---|---|
| macOS | 26.6.2 (25G83), arm64 | — |
| Homebrew | 7.0.10 | Ya instalado |
| Git | 2.50.1 (Apple Git-155) | Ya instalado |
| Rust / Cargo | 1.97.1 / 1.97.1 (rustup 1.29.0) | Ya instalado; falta el target `wasm32-wasip1` (necesario en la Fase 1) |
| Ghostty | 1.3.1 (stable) | Ya instalado |
| Zellij | 0.45.1 | Instalado en esta fase (`brew install zellij`) |
| Micro | 2.0.15 | Instalado en esta fase (`brew install micro`) |
| Claude Code | 2.1.304 | Ya instalado |
| Codex CLI | 0.163.0 | Ya instalado |

No se usó `sudo` ni scripts remotos. No se modificaron `~/.config/ghostty`,
`~/.config/zellij`, `~/.config/micro` ni los rc del shell.

## 3. Pruebas automatizadas (`scripts/smoke-test.py`)

| Modo | Qué hace | Resultado |
|---|---|---|
| Configuración | `ghostty +validate-config`; configuración y atajos efectivos de Ghostty con un `XDG_CONFIG_HOME` aislado; `zellij setup --check` | 8/8 |
| PTY | `bin/vkterm` en un pseudo-terminal 200×50. Envía los bytes que Ghostty emite para cada atajo ⌘, leídos de su configuración efectiva | 68/68 |
| `--ghostty` | Ghostty real lanzado con `bin/vkterm-ghostty --new`. Eventos de teclado CGEvent entregados al proceso de Ghostty, sin robar el foco | 39/39 |
| Sesiones previas | Las sesiones que existían antes de la prueba siguen vivas | 1/1 |

**Total: 116/116 en dos ejecuciones consecutivas** de
`scripts/smoke-test.py --ghostty` con el código final de la corrección 2
(la corrección 1 dio 107/107 en tres ejecuciones).

Cubre:

- **Layout**: sin paneles apilados, tres terminales visibles, proporciones
  20/45/35.
- **Agentes**: Claude Code y Codex en espera, sin proceso.
- **Editor**: abre `Header.tsx`.
- **Foco**: ⌘1–5 desde varios orígenes, ⌘⌥ flechas con ⌥ izquierdo y derecho,
  ⌥ derecho + h/l/1/z. ⌥ izquierdo escribe caracteres (`™@`) y no navega.
- **Guardar y abrir**: ⌘S y Ctrl-S guardan (comprobado en disco). ⌘P prepara
  `tab` en el editor desde el Shell y abre el archivo en una pestaña nueva.
- **Copiar y pegar**: ⌘V pega en Micro; en el shell no ejecuta lo pegado. ⌘C
  no interrumpe procesos ni toca el portapapeles.
- **Teclas de macOS**: ⌘S no congela la salida del shell. ⌘← → van a
  inicio / fin de línea en Micro y en zsh. ⌘⌫ no activa macros en Micro.
- **Explorador → editor central** (PTY y Ghostty): Enter abre el archivo como
  pestaña del Micro central y le da el foco; no quedan paneles nuevos ni otro
  Micro; reabrir salta a la pestaña existente; Strider vuelve a la carpeta con
  una entrada seleccionada (Enter no abre un terminal); los binarios no se
  abren; ⌥ derecho + , / . cambian de pestaña; `zellij edit` desde el shell
  abre en el editor central y `$EDITOR` sigue siendo `micro`.
- **Procesos**: cada panel tiene su propio proceso y PTY; matar el shell de
  uno no afecta a los demás.
- **Sesiones**: desconectar y reanudar, `--new`, cerrar y recrear con el mismo
  layout, y sesiones previas intactas.

Garantías de la suite: nunca ejecuta Claude Code ni Codex. El portapapeles se
compara antes y después y no se modifica. En la prueba de ⌘V, su contenido se
pega en una copia temporal, se deshace sin guardar y no se muestra.

## 4. Verificación de la configuración efectiva

- **Ghostty.** `+show-config` no acepta `--config-file`, así que se ejecuta con
  un `XDG_CONFIG_HOME` temporal cuya config solo incluye el perfil. Equivale a
  lo que hace el lanzador: valores por defecto + perfil, sin la config global.
  Resultado: `macos-option-as-alt = right`, `title = vkTERM`, tema Catppuccin
  Mocha, `maximize = true`, y nada de `~/.config/ghostty`. `+list-keybinds`
  confirma los 20 atajos traducidos y que ⌘T, ⌘D y ⌘⇧D están desasignados.
  En la instancia real, los argumentos del proceso incluyen
  `--config-default-files=false` y la ruta del perfil.
- **Zellij.** `setup --check` da "Well defined". La sesión arranca en LOCKED
  con el layout de tres terminales. Tras `kill-session`, el mismo comando la
  recrea con el mismo nombre, layout y archivo inicial.
- **Permiso del disco externo.** Ghostty ya arranca desde
  `/Volumes/Apps/...`, lo que indica que se concedió el permiso de macOS.

## 5. Pendiente

Requiere plugin (Fase 1):

1. **Enter sin selección en el explorador abre un terminal.** Es una función
   de Strider (`open_terminal`) que no pasa por `scrollback_editor`. Mitigado
   solo tras abrir un archivo.
2. **Enfocar paneles por nombre.** ⌘2–⌘5 son posicionales. Zellij 0.45.1 no
   tiene acción de keybinding para enfocar por ID.
3. **⌘C sobre la selección interna de Micro.** Ghostty no sabe qué
   aplicación tiene el foco y enviar Ctrl-C rompería los shells.
4. **Explorador en árbol**, sin el panel temporal de `vkterm-open` (visible
   ≈0,3 s al abrir un archivo).

Sin plugin:

- **Pruebas con teclado físico** (`docs/manual-tests.md`), incluida la capa de
  atajos globales de macOS, que las pruebas sintetizadas no atraviesan.
- **⌘S y ⌘P con Claude Code / Codex en ejecución**: no probados.
- **⌘⌫ en Micro**: hoy no hace nada. Falta "borrar hasta el inicio de línea":
  la cadena `SelectToStartOfLine&Delete` borra un carácter de más en la
  columna 0.
- **⌘K, ⌘F, ⌘Z, ⌘A**: decidir si se traducen.
- **Atajos ⌘ para las pestañas del editor** (hoy ⌥ derecho + , / . y clic):
  decidir cuáles. ⌘⌥ ← → ya están ocupados por la navegación entre paneles.
- **⌘P con un archivo ya abierto** abre una segunda pestaña del mismo archivo
  (mismo búfer, sin riesgo de perder cambios). Desde el explorador no pasa.
- Confirmar la licencia (MIT provisional) e instalar `wasm32-wasip1`.

## 6. Problemas técnicos encontrados

1. **Conflictos Ctrl entre Zellij y Micro / zsh / agentes.** Resueltos con el
   modo locked.
2. **Ghostty y el disco USB externo.** Sin el permiso de macOS, Ghostty se
   bloqueaba al leer el perfil (diagnosticado con `sample`). Ya concedido.
3. **AppKit trata las rutas sueltas de argv como archivos a abrir.** Resuelto
   pasando los argumentos por `open --env`.
4. **PATH mínimo de Ghostty.** Resuelto con `zsh -l`.
5. **Micro anunciaba "Ctrl-g: help" al azar.** Ctrl g desasignado en Micro.
6. **Estado de Micro dentro de `config/micro`.** Ignorado por git.
7. **Validación anterior insuficiente** (corrección 1). Inyectaba bytes en el
   PTY sin pasar por Ghostty, así que no podía detectar fallos de teclado
   reales. Ahora la suite incluye Ghostty real.
8. **Ctrl-S congela la salida en los shells.** Por eso ⌘S envía F2. Efecto
   menor: con un comando en marcha, el tty muestra `^[OQ`.
9. **Inyección de teclas y foco.** La primera versión del inyector enviaba
   eventos al sistema y exigía que Ghostty estuviera al frente. Con otra app
   activa (Firefox), la guarda abortó como estaba previsto. Ahora los eventos
   se entregan solo al proceso de Ghostty (`postToPid`): no roban el foco ni
   pueden llegar a otra app, pero tampoco atraviesan los atajos globales.
10. **`ghostty +show-config` no acepta `--config-file`.** Se usa un
    `XDG_CONFIG_HOME` aislado.
11. **Apertura de archivos de Strider** (corrección 2):
    - Zellij abre los archivos de los plugins con `scrollback_editor` (o
      `$EDITOR`), le pasa la ruta absoluta y cierra el panel cuando el
      programa termina. Eso permite redirigirlos sin plugin.
    - Strider "entra" en el archivo que abre: la lista queda vacía y un Enter
      abre un terminal. Por eso `vkterm-open` le envía ← ↓.
    - Esc seguido inmediatamente de Ctrl-E llega a Micro como Alt-Ctrl-E: se
      separan 0,1 s.
    - Si un panel se encoge, `dump-screen` devuelve también filas antiguas por
      debajo: la barra de estado de Micro se busca de arriba abajo.
    - Con ← en la raíz del proyecto, Strider sube a las carpetas superiores
      (así se llegó a `vkmeet` en la captura de la revisión).
12. Notas para las herramientas:
    - `dump-screen` no sirve para plugins ni flotantes; `subscribe` sí.
    - Los paneles plegados de un stack figuran como `is_suppressed`.
    - Un zsh recién abierto puede descartar entrada.
    - Strider deja la selección vacía al entrar en una carpeta.

## 7. Recomendación para la Fase 1 (sin cambios)

Plugin propio `plugins/explorer` (Rust, `zellij-tile` 0.45.1,
`wasm32-wasip1`) que sustituya a Strider:

- Árbol del proyecto.
- Apertura en el panel Editor. La corrección 2 ya valida la estrategia de
  enviar el archivo al Micro existente (`tab` / `tabswitch`); el plugin la
  invocaría directamente, sin panel temporal ni terminal al pulsar Enter sin
  selección.
- Enfoque por nombre (Alt 2–5 pasarían a ser mensajes al plugin, sin cambiar
  la capa ⌘ de Ghostty).
- Ampliar `scripts/smoke-test.py`.

Antes de empezar: aprobar esta corrección e instalar `wasm32-wasip1`.
