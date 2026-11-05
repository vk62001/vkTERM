# Atajos de teclado

## Dos modos por pestaña

Cada pestaña (o división) de vkTERM.app está en uno de dos modos:

- **Modo terminal**: un terminal como cualquier otro. Los atajos son los de
  Ghostty (pestañas, divisiones, buscar, limpiar, cerrar), salvo cinco: ⌘B,
  ⌘J, ⌘P y ⌘⇧P convierten la pestaña en el IDE, y ⌘O elige una carpeta ya
  abierta antes («Abrir reciente»).
- **Modo IDE**: el workspace tipo VS Code. Los atajos ⌘ de vkTERM van al
  explorador, el editor y los paneles.

El modo es una *tabla de atajos* de Ghostty 1.3 (`vkterm/…` en
`config/ghostty/vkterm.ghostty`). bin/vkterm la activa en su pestaña por
AppleScript (`perform action "activate_key_table:vkterm"`) al convertirla y
la desactiva al cerrar la carpeta (⌘K F). Ghostty busca primero en la tabla
activa y después en la de por defecto, así que en el IDE siguen funcionando
⌘T, ⌘⇧N, ⌘Q y los demás atajos de Ghostty que vkTERM no usa. Mientras la
tabla está activa, la barra de estado del IDE empieza por «⌨ vkTERM». El
indicador flotante que Ghostty 1.3 dibuja para las tablas no aparece si
vkTERM.app se creó con Ghostty compilado por `scripts/build-ghostty.sh`; con
la Ghostty.app instalada tal cual, se ve abajo, encima de la barra de estado.

## Modo terminal

| Atajo | Acción | Implementación | Verificado |
|---|---|---|---|
| ⌘B · ⌘J · ⌘P · ⌘⇧P | En el prompt de zsh: convierten la pestaña en el IDE de la carpeta actual y después muestran el explorador, el Shell (con el foco), la búsqueda de archivos o la paleta de comandos. En `~` y `/` no hacen nada (aviso abajo). Lo que se estaba escribiendo vuelve al prompt al cerrar la carpeta. | F13–F16 con Super (`csi:25;9~`, `26;9~`, `28;9~`, `29;9~`) → widget de zsh (config/zsh/vkterm.zsh) → `vkterm --then ACCIÓN .` | G (⌘P, ⌘J), P (los cuatro, modo vi) |
| ⌘O · `vkterm -r` | «Abrir reciente», como «File: Open Recent» de VS Code (allí ⌃R; aquí ⌃R es la búsqueda del historial de zsh): las carpetas ya abiertas en vkTERM, la última usada primero, con su carpeta de arriba y «abierta» o «no disponible». Escribir filtra; Enter la abre en esta pestaña (si ya está abierta en otra, trae aquí su IDE), ⇧Enter en una ventana nueva (si ya está abierta, va a su pestaña). Esc, ⌃C u otra vez ⌘O cierran. También en `~`. | F20 con Super (`csi:34;9~`) → widget de zsh → `vkterm -r` → bin/vkterm-recent (la lista debajo del prompt, con el protocolo de teclado de kitty para distinguir ⇧Enter) → bin/vkterm-state recent | P (zsh, la lista en un PTY); Ghostty real pendiente (K16) |
| `vkterm .` · `vkterm RUTA` | Convertir la pestaña en el IDE de esa carpeta (`vkterm` sin nada es `vkterm .`). Si ya está abierta en otra pestaña o ventana, trae aquí su IDE con todo vivo (Claude Code, Codex, el Shell, los archivos sin guardar); la otra vuelve a su shell y lo dice. Una carpeta se abre una sola vez. `--window`: en una ventana nueva. | bin/vkterm → bin/vkterm-surface detach (⌥` en la otra pestaña → Detach de Zellij) → bin/vkterm-workspace --attach | P (Zellij en un PTY); Ghostty real pendiente (K21) |
| ⌘T · ⌘N (o ⌘⇧N) · ⌘D · ⌘⇧D | Pestaña, ventana y divisiones de Ghostty | Ghostty; ⌘T y ⌘N por tecla física (`super+key_t`, `super+key_n`) | G (⌘T) |
| ⌘W | Cerrar la pestaña de Ghostty | Ghostty | G |
| ⌘1 – ⌘9 · ⌃Tab | Ir a una pestaña de Ghostty | Ghostty | — |
| ⌘K · ⌘F · ⌘Z · ⌘A · ⌘C | Limpiar, buscar, deshacer, seleccionar y copiar, como en Ghostty | Ghostty | — |
| ⌘V | Pegar, como en Ghostty. En Claude Code y Codex, si el portapapeles solo tiene una imagen, la adjunta (como su ⌃V), igual que en el IDE. | `performable:super+v=paste_from_clipboard`: sin texto, Ghostty no pega y envía la tecla (Super+V en el protocolo de kitty). `claude` y `codex` corren a través de bin/vkterm-agent, que la convierte en ⌃V | Ghostty real con teclas sintetizadas (`[Image #1]` en Claude Code); K14 |

Fuera del prompt (con vim, less o ssh en primer plano), ⌘B, ⌘J, ⌘P, ⌘⇧P y
⌘O llegan a ese programa como F13–F16 y F20, que normalmente ignora.

## Cómo llega una tecla a cada programa (modo IDE)

```
teclado ─▶ macOS ─▶ Ghostty ─────────────▶ Zellij ──────────────────▶ plugin vkterm / aplicación
                    · ⌘: los traduce        · modo LOCKED: deja pasar   · ⌘B, ⌘J, ⌥⌘B, ⌘1–5, pestañas…
                      (perfil vkterm)         casi todo                 · Micro, zsh, Claude Code, Codex
                    · ⌥ (los dos) =         · capa Alt de vkTERM
                      caracteres              siempre activa
                                            · Ctrl g: modo normal
```

- **⌘ Command**: ni Zellij ni Micro ven la tecla Command. Ghostty captura cada
  atajo ⌘ de vkTERM y lo traduce a:
  - un código Alt (`esc:`) que Zellij convierte en un mensaje al plugin
    `vkterm` (`MessagePlugin`);
  - Home / End (`csi:`);
  - F2 (Guardar en Micro).
- **⌥ Option**: los **dos** escriben los caracteres del teclado Spanish - ISO,
  como en cualquier app de macOS y en VS Code: ⌥2 = @, ⌥3 = #, `[ ] { } | \ ~`
  (`macos-option-as-alt = false`). Las combinaciones que no producen un
  carácter siguen llegando como Alt: ⌥← / ⌥→ saltan palabra en el editor.
- **⌃ Control**: llega tal cual. Zellij está en modo **locked**, así que casi
  todo pasa a la aplicación. La excepción es **Ctrl g**, que desbloquea Zellij.

La columna *Verificado* indica cómo se probó cada atajo:

- **G**: en Ghostty real, con eventos de teclado sintetizados entregados al
  proceso de Ghostty (`smoke-test.py --ghostty`). Recorren Ghostty → Zellij →
  plugin / aplicación, pero **no son pulsaciones físicas** y no pasan por los
  atajos globales de macOS.
- **P**: en el PTY, con los bytes exactos que emite Ghostty según su
  configuración efectiva (`smoke-test.py`).
- **M**: pendiente de la prueba manual con teclado físico
  ([manual-tests.md](manual-tests.md)).

## Modo IDE

| Atajo | Acción | Implementación | Verificado |
|---|---|---|---|
| ⌘K F | Cerrar la carpeta (como «File: Close Folder» de VS Code): termina la sesión (Micro, Claude Code, Codex) y vuelve al shell de antes, intacto. Si se perdería algo (archivos sin guardar, Claude Code o Codex en marcha), pregunta: Guardar y cerrar · Cerrar sin guardar · Cancelar. | Secuencia de Ghostty → Alt q → `close_folder`; lo modificado lo escribe el plugin Lua de Micro; confirmación en otra instancia del plugin (`mode "confirm"`) | G, P |
| ⌘O | «Abrir reciente» (como «File: Open Recent» de VS Code): las otras carpetas ya abiertas en vkTERM, la última usada primero; escribir filtra. Enter abre la elegida en esta pestaña: cierra antes la actual, preguntando como ⌘K F si se perdería algo. Si ya está abierta en otra pestaña, Enter trae aquí su IDE. ⇧Enter la abre en una ventana nueva (o va a su pestaña, si ya está abierta). También «Archivo: Abrir reciente…» en ⌘⇧P. | Alt b → mensaje `toggle_recent` a otra instancia del plugin (`mode "recent"`, lista de bin/vkterm-state recent) → mensaje `palette` `open_recent` al explorador: con Enter, `close_folder` y la carpeta en `projects/<huella>/next`, que bin/vkterm abre al terminar la sesión; con ⇧Enter, `vkterm --window` | P (Enter, ⇧Enter, la confirmación); Ghostty real pendiente (K17) |
| ⌘T | Pestaña nueva de Ghostty, en modo terminal, en la carpeta del proyecto | Tabla por defecto (bin/vkterm emite OSC 7 con la carpeta) | G |
| ⌘N | Nuevo archivo (como «Nuevo archivo…» del explorador de VS Code): pide el nombre y lo crea vacío en la carpeta seleccionada del explorador (o en la del archivo seleccionado); con `/` crea también las carpetas. Avisa si ya existe. Enter lo crea, lo selecciona en el explorador y lo abre en una pestaña; Esc cancela. También «Archivo: Nuevo archivo…» en ⌘⇧P. | Alt d → `new_file` → el explorador abre otra instancia del plugin (`mode "newfile"`) con la carpeta → mensaje `palette` `create_file` → el explorador lo crea (en `/host`) y lo abre (bin/vkterm-open) | G, P |
| ⌘⇧N | Ventana nueva de vkTERM, en modo terminal (como VS Code) | Tabla por defecto (`super+shift+key_n`) | P (configuración) |
| ⌘⇧, | Recargar la configuración de Ghostty sin salir del modo IDE | Alt r → `reload_config` → bin/vkterm-theme reload → bin/vkterm-surface reactivate | G |
| ⌘⇧D | Nada: Zellij gestiona los paneles del IDE (⌘D es el multicursor) | `ignore` | P (configuración) |
| ⌘B | Mostrar / ocultar la barra lateral, con la vista que tenga (explorador o Control de código fuente) | Alt 6 → plugin `toggle_sidebar` | G, P, M |
| ⌃⇧G | Control de código fuente, como en VS Code: la barra lateral cambia de vista (si está oculta, se muestra) y el foco va al mensaje del commit. Ver abajo. | `ctrl+shift+g=esc:/` → Alt / → `show_view scm` | P; Ghostty real pendiente (R1) |
| ⌘⇧E | Volver al explorador (la otra vista de la barra lateral), con el foco | `super+shift+e=esc:'` → Alt ' → `show_view explorer` | P; Ghostty real pendiente (R1) |
| ⌘Enter | Commit, en Control de código fuente con el foco (fuera de ella no hace nada). En el modo terminal no hace nada; la pantalla completa de Ghostty queda en ⌃⌘F. | `super+enter=esc:*` → Alt * → `scm_commit`. En la tabla por defecto, `super+enter=ignore`: con ⌘Enter en el menú de Ghostty (pantalla completa), macOS lo resolvería antes que la tabla del IDE | P; Ghostty real pendiente (R4) |
| ⌘J | Mostrar / ocultar el Shell (panel inferior). Al mostrarlo recibe el foco. | Alt 7 → `toggle_panel` | G, P, M |
| ⌥⌘B | Mostrar / ocultar Claude Code y Codex (barra derecha), con cualquiera de los dos ⌥ | Alt 8 → `toggle_agents` | G, P, M |
| ⌘1 – ⌘5 | Foco en Explorador, Editor, Claude Code, Codex, Shell. Si el panel está oculto, lo muestra. | Alt 1–5 → `focus` por identidad del panel | G, P, M |
| ⌘⌥← / ⌘⌥→ | Pestaña anterior / siguiente del editor (y foco en el editor) | Alt , / Alt . → `prev_tab` / `next_tab` → Alt-, / Alt-. de Micro | G, P, M |
| ⌃⇧Tab / ⌃Tab | Igual que ⌘⌥← / ⌘⌥→ | Alt , / Alt . | G, P, M |
| ⌘W | Cerrar la pestaña del editor. Fuera del editor no hace nada (ya no cierra la ventana). | Alt 9 → `close_tab` → Ctrl-Q de Micro | G, P, M |
| ⌘P | Paleta de archivos (como Quick Open de VS Code): busca por nombre mientras escribes, Enter abre en una pestaña (o salta a ella si ya está abierta), Esc cierra. | Alt 0 → `palette_files` a otra instancia del plugin (`mode "palette"`), flotante. Archivos de `git ls-files` (o recorrido del proyecto sin git), búsqueda difusa en `src/fuzzy.rs`; Enter → mensaje `palette` al explorador → `open` (bin/vkterm-open). | G, P |
| ⌘⇧P | Paleta de comandos: todos los comandos de vkTERM con su atajo; `>` delante en la misma ventana. | Alt ; → `palette_commands`; lista en `src/commands.rs`; el explorador los ejecuta cuando la ventana se ha cerrado. | G, P |
| ⌘S | Guardar en Micro | F2 | G, P, M |
| ⌘K ⌘T | Tema de color (como VS Code): agrupados en claros, oscuros y alto contraste; escribir filtra; ↑ ↓ prueban cada tema, Enter lo deja, Esc vuelve al anterior. | Secuencia de Ghostty → Alt t → mensaje `toggle_themes` a otra instancia del plugin (`mode "themes"`); el explorador lo aplica (`set_theme`) | G, P |
| ⌘K ⌘L | Fuente: las monoespaciadas instaladas; escribir filtra; ↑ ↓ prueban cada una, ← → cambian el tamaño, Enter la deja, Esc vuelve a la anterior. VS Code no tiene uno (la L es de «letra»). | Secuencia de Ghostty → Alt g → mensaje `toggle_fonts` a otra instancia del plugin (`mode "fonts"`); el explorador la aplica (`set_font`, `font_size`, bin/vkterm-font) | G, P |
| ⌘+ · ⌘- · ⌘0 | Tamaño de letra: más, menos y el normal (13 pt), como «View: Zoom» de VS Code; en todas las ventanas y se recuerda. Con el teclado español, el + y el - de su tecla; con el de EE. UU., ⌘= y ⌘-. También en el prompt del modo terminal. | Ghostty por carácter → Alt m / Alt w / Alt e → mensaje `font_size` (+1, -1, 0) → bin/vkterm-font | G, P |
| ⌘K ⌘S | Ventana de atajos (como Unity), con los colores del editor del tema y las teclas en etiquetas, como el editor «Keyboard Shortcuts» de VS Code. Se cierra con Esc, cualquier tecla, un clic, otra vez ⌘K ⌘S o al pasar el foco a otro panel. | Secuencia de Ghostty → Alt ? → mensaje `toggle_shortcuts` a otra instancia del plugin (`mode "shortcuts"`) en un panel flotante | G, P |
| ⌘⌥↑ / ⌘⌥↓ | Foco al panel de arriba / abajo (editor ↔ Shell) | Alt k / j → `MoveFocus` | G, P, M |
| ⌘Z / ⌘⇧Z | Deshacer / rehacer en el editor | Alt u / Alt y → `edit` → Ctrl-Z / Ctrl-Y a Micro, solo con el foco en el editor | P, M |
| ⌘A | Seleccionar todo en el editor | Alt a → `edit` → Ctrl-A | P, M |
| ⌘F | Buscar en el archivo | Alt s → `edit` → Ctrl-F | P, M |
| ⌘X | Cortar en el editor (sin selección, la línea) | Alt x → `edit` → Ctrl-X (Cut\|CutLine) | P (no actúa en el shell), M |
| ⌘C | Copiar: la selección de Ghostty y, en el editor, la de Micro (sin selección, la línea). Nunca envía Ctrl-C a un shell. | Acción de Ghostty encadenada con Alt c → `edit` → Ctrl-C (Copy\|CopyLine) | G (no interrumpe procesos), P, M |
| ⌘V | Pegar. En Claude Code y Codex, si el portapapeles solo tiene una imagen, la adjunta (como su ⌃V). | Acción de Ghostty (bracketed paste) encadenada con Alt v → `paste`: con el foco en un agente, `osascript -e 'clipboard info'` y, si es solo una imagen, ⌃V a ese panel | G (Micro), P (Micro y shell), M |
| Clic derecho en el editor | Menú contextual, como el de VS Code: Cortar, Copiar y Pegar (sin selección, Cortar y Copiar toman la línea), Seleccionar todo y Paleta de comandos…, con su atajo a la derecha. Se abre junto al puntero; fuera de la selección, el cursor va antes al sitio del clic. ↑ ↓ y Enter, o el ratón; Esc o un clic fuera lo cierran. | Ghostty → Zellij → Micro (`MouseRight` → plugin Lua: coloca el cursor y `zellij pipe` `context_menu`) → el explorador abre otra instancia del plugin (`mode "menu"`) ya colocada; lo elegido son las teclas de Micro (Ctrl-X/C/V/A) en ese panel | P (clic SGR en Zellij), M (Q1–Q6) |
| ⌘. · ⇧F10 | El menú del clic derecho del explorador (Nuevo archivo…, Renombrar…, Mostrar en Finder, mostrar u ocultar .git, Paleta de comandos…) para lo seleccionado, debajo de ello. ⌘. desde cualquier panel (muestra el explorador y le da el foco); ⇧F10, como en VS Code, con el foco en el explorador. Zellij 0.45.1 no les pasa el clic derecho a los plugins. | Ghostty `esc::` → Alt : → mensaje `explorer_menu`; ⇧F10 llega al explorador como tecla → el mismo menú (`mode "menu"`) | P; Ghostty real y M (Q7, Q8) pendientes |
| ⌘← / ⌘→ | Inicio / fin de línea | Home / End | G (Micro), P (Micro y shell), M |
| ⌘D · ⌘/ · ⌘] · ⌘[ · ⌘G · ⌘⇧G · ⌘L · ⌘⇧K | Edición avanzada, como VS Code: cursor en la siguiente coincidencia (multicursor), comentar la línea, indentar / quitar sangría, siguiente / anterior coincidencia de la búsqueda, seleccionar y borrar la línea. Solo con el editor enfocado. ⌘/ también como ⌘⇧7 (teclado español); ⌘] y ⌘[ son las teclas físicas de esos símbolos en el teclado de EE. UU. | Ghostty `esc:!` `esc:#` `esc:$` `esc:%` `esc:&` `esc:(` `esc:)` `esc:<` → mensaje `edit` → Alt-n, Ctrl-/, Alt-$ (plugin Lua `indent`), Alt-% , Alt-&, Alt-(, Alt-), Alt-< de Micro (config/micro/bindings.json) | P (en un Micro real); Ghostty real pendiente |
| ⌘⇧P → Historial · Blame · Dividir · Ir a la línea · Autoguardado · Iniciar repositorio | «Git: Historial del archivo» y «Git: Blame del archivo» abren `vkterm-git log` / `blame` en una ventana flotante; «Editor: Dividir a la derecha / abajo» (`vsplit` / `hsplit` del mismo archivo); «Editor: Ir a la línea…» (`goto`, escribes el número); «Archivo: Activar el autoguardado» (`set autosave 2`, solo esta sesión) | Comandos del plugin Lua vkterm de Micro; `Git: Iniciar repositorio` = `git init` | P (git, split) |
| ⌘↑ · ⌘↓ | Principio / fin del archivo | `csi:1;5H` / `csi:1;5F` (Ctrl-Inicio / Ctrl-Fin) | P |
| ⌥↑ · ⌥↓ · ⌥⇧↑ · ⌥⇧↓ | Mover y duplicar la línea (las teclas de Micro) | Alt+flecha a Micro; `AltShiftDown` = DuplicateLine | P |
| ⌘⇧F | Buscar en todo el proyecto: escribir busca (ripgrep, `git grep` si no está; respeta .gitignore, ignora mayúsculas salvo que escribas alguna). ↑ ↓ eligen; Enter abre el archivo en esa línea. Tab añade el campo «Reemplazar»: Enter pide confirmación («reemplazar en N archivos») y otro Enter reemplaza, literalmente, en todos. Los archivos abiertos con cambios sin guardar no se avisan: Micro ofrece recargarlos. | Alt { → `toggle_search` a otra instancia del plugin (`mode "search"`, src/search.rs); Enter → mensaje `palette` `open` con ruta y línea → bin/vkterm-open (`VKTERM_LINE` → `goto N` de Micro) | P (lógica); Ghostty real pendiente |
| F12 · ⇧F12 | Ir a la definición · buscar todas las referencias | F12 directo a Micro (`command:lsp goto-definition`); ⇧F12: Ghostty `esc:\|` → `edit` → `lsp find-references` | P (µlsp en Micro); Ghostty real pendiente |
| ⌘I · ⌘K ⌘I | Sugerencias · información del símbolo | `esc:>` / `esc:^` → `edit` → `lsp autocomplete` / `lsp hover` en la línea de comandos de Micro (Ctrl-E) | pendiente |
| ⌘K ⌘R · ⌘K ⌘F · ⌘⇧O | Cambiar el nombre del símbolo · dar formato al documento · símbolos del archivo | `esc:@` `esc:~` `esc:}` → `lsp rename` / `format` / `document-symbols` | pendiente |
| ⌘K Z | Ampliar / restaurar el panel con el foco (como el modo Zen de VS Code) | Secuencia de Ghostty → Alt z | G, P |
| ⌘K ⌘C | Copiar todo el historial del panel con el foco (Claude Code, Codex, Shell) | Secuencia de Ghostty → Alt + → `zellij action dump-screen --full` | G, P |
| ⌥2, ⌥3, ⌥ + tecla | @, # y el resto de caracteres del teclado, con cualquiera de los dos ⌥ | Carácter de macOS | G (⌥2, ⌥3), M |
| ⌥← / ⌥→ | Palabra anterior / siguiente en el editor | Alt+flecha → Micro | G, M |
| Ctrl g | Desbloquear Zellij (modo normal) | Preset Unlock-First | G, P, M |

**Ocultar no cierra nada.** El proceso de cada panel sigue vivo (la
conversación con Claude Code no se pierde) y, al mostrarlo, vuelve a su sitio
con su tamaño. Si redimensionas paneles a mano, al ocultar o mostrar se
recuperan los tamaños del layout (pendiente, H3).

Los códigos internos (Alt 0–9, Alt , / ., Alt ?, Alt t, Alt z, Alt h/j/k/l,
Alt u/y/a/s/x/c) son la
capa que Ghostty alcanza con ⌘. En otro terminal con Option como Alt también
se activan con Alt.

**⌘K ⌘S es una secuencia,** como en VS Code: ⌘K y después ⌘S, sin soltar ⌘
o soltándolo entre medias. Ghostty espera la segunda tecla sin límite de
tiempo. Si es otra, envía las dos al programa como si no hubiera atajo. Un ⌘S
suelto sigue guardando. En el modo IDE, ⌘K solo (limpiar pantalla en
Ghostty) no existe; en el modo terminal, sí.


### En Control de código fuente (⌃⇧G)

| Tecla | Dónde | Acción |
|---|---|---|
| ↑ ↓ · Re Pág · Av Pág · Inicio · Fin | Toda la vista | Mover la selección: mensaje, comparación, grupos y archivos |
| Escribir · ⌫ · Ctrl u | Mensaje | Escribir el mensaje, borrar, vaciarlo |
| Enter · ⌘Enter | Mensaje | Commit de lo preparado; sin nada preparado, pregunta «¿Preparar todos los cambios y hacer commit?» |
| Enter | Comparación (⇄ main) | Elegir la rama con la que comprobar los conflictos (se recuerda por carpeta) |
| Enter · ← · → | Cabecera de un grupo | Plegar o desplegar |
| Espacio | Cabecera de un grupo | Preparar todo el grupo («Cambios», «Conflictos de fusión») o quitar todo («Cambios preparados») |
| ⌫ | Cabecera de «Cambios» | Descartar todos los cambios y borrar los archivos nuevos (pregunta antes) |
| Enter · clic | Archivo | Diferencias en una ventana flotante con el paginador de git (q la cierra). En un conflicto que habría con otra rama, el resultado de la fusión con sus marcas `<<<<<<<` |
| Espacio | Archivo | Preparar o quitar; en un conflicto, marcarlo como resuelto (pregunta si aún tiene marcas) |
| → | Archivo | Abrirlo en una pestaña del editor |
| ⌫ | Archivo de «Cambios» | Descartar sus cambios, o borrarlo si es nuevo (pregunta antes) |

Pull, Push, Sincronizar, Fetch, Comparar con otra rama, Abortar la fusión y
Actualizar están en ⌘⇧P («Git: …»).

## Explorador

Árbol del proyecto, como el de VS Code. La raíz es la carpeta del proyecto: no
se puede subir más arriba. Nunca abre terminales.

| Tecla | Acción |
|---|---|
| ↑ ↓ | Mover la selección |
| → | Desplegar la carpeta; si ya lo está, bajar a su primer hijo |
| ← | Plegar la carpeta; si no, subir a la carpeta padre |
| Enter | Archivo: abrirlo en una pestaña del editor y pasar el foco al editor. Carpeta: plegar / desplegar |
| Espacio | Abrir el archivo sin quitar el foco del explorador |
| Inicio / Fin, Re Pág / Av Pág | Saltar |
| Una letra | Siguiente entrada que empieza por ella |
| Ctrl h | Mostrar / ocultar `.git`, `.DS_Store` y demás excluidos de VS Code |
| ⌘. · ⇧F10 | Menú del clic derecho de lo seleccionado (el clic derecho no llega al explorador con Zellij 0.45.1; ver arriba) |
| Clic | Seleccionar y abrir (archivo) o plegar / desplegar (carpeta) |
| Rueda | Desplazar la lista |

- Si el archivo ya está abierto, salta a su pestaña en lugar de abrir otra.
- Los binarios (con bytes NUL, como Git) no se abren: aviso en la última fila.
- El árbol se refresca solo al crear, borrar o renombrar archivos.
- Con el editor sin pestañas (vacío), el archivo ocupa su lugar.

Verificado (P y G): ↑ ↓ ← →, Enter, Espacio, letra, Inicio, binarios, raíz
fija. Clic verificado solo en P, con mensajes de ratón al cliente. La rueda,
Re Pág / Av Pág, Ctrl h y el refresco automático no tienen prueba automática.

`zellij edit <archivo>` desde un terminal de la sesión también abre en el
editor central. `$EDITOR` sigue siendo `micro`: git y los agentes abren Micro
en su propio panel.

## Editor (Micro)

⌘S o Ctrl s guardar · ⌘P abrir por nombre · Ctrl o abrir en la pestaña
actual · Ctrl f buscar · Ctrl z / Ctrl y deshacer / rehacer · ⌥ ← / → palabra
· ⇧ + flechas seleccionar · Ctrl c copiar la selección de Micro · **F1
ayuda** · Alt g menú de atajos · clic derecho: menú contextual (Cortar,
Copiar, Pegar, Seleccionar todo, Paleta de comandos…), en cualquier Micro de
la sesión.

Con más de un archivo abierto aparece la barra de pestañas (la activa entre
corchetes); también se cambia con un clic. ⌘W o Ctrl q cierran la pestaña. Al
cerrar la última, Micro vuelve a abrirse vacío, como el editor de VS Code sin
pestañas.

## Tras Ctrl g (modo normal de Zellij)

| Tecla | Modo / acción |
|---|---|
| p | **Panel**: n nuevo · d abajo · r derecha · x cerrar · f pantalla completa · w flotantes · c renombrar · h/j/k/l foco |
| t | **Pestaña**: n nueva · x cerrar · r renombrar · 1–9 ir a · h/l anterior/siguiente |
| r | **Redimensionar**: h/j/k/l agrandar · H/J/K/L reducir |
| s | **Scroll**: j/k · d/u · f buscar · e abrir el scrollback en una pestaña del editor |
| o | **Sesión**: w gestor de sesiones. Sin d (desconectar): la sesión dura lo que su pestaña |
| Ctrl q | Cerrar Zellij y todas las terminales de la sesión |
| Esc / Enter / Ctrl g | Volver a locked |

## Paneles de agentes

"Waiting to run": **Enter** ejecuta `claude` / `codex` · **Esc** abre un shell
en ese panel · **Ctrl c** cierra el panel.

## Conflictos encontrados

| Atajo | Conflicto | Resolución | Estado |
|---|---|---|---|
| ⌘W | En Ghostty cierra la pestaña o la ventana. | Modo IDE: cierra la pestaña del editor; fuera del editor no hace nada. Modo terminal: el de Ghostty. | Resuelto. |
| ⌘J | Ghostty: desplazarse a la selección. | Panel (Shell), como VS Code. | Resuelto. |
| ⌃Tab / ⌃⇧Tab | Pestañas de Ghostty. | Pestañas del editor. | Resuelto. |
| ⌘⌥ flechas | Ghostty: navegar entre sus splits. En la corrección 1 movían el foco entre paneles. | ← → cambian de pestaña (VS Code, decisión del usuario); ↑ ↓ siguen moviendo el foco. | Resuelto. |
| Explorador: Enter sin selección | Strider abría un terminal junto al explorador. | Strider se sustituye por el plugin vkterm, que nunca abre terminales. | Resuelto. |
| Explorador: primer clic | Si el explorador no tiene el foco, Zellij usa el primer clic solo para enfocarlo; el segundo abre. | — | Abierto, menor (comportamiento de Zellij). |
| ⌘S | Enviar Ctrl-S congela la salida de los comandos en el shell (tty con `ixon`). | ⌘S envía F2. | Resuelto. Efecto menor: con un comando en marcha en el shell, el tty muestra `^[OQ`. |
| ⌘S, ⌘P, ⌘W con agentes | No se han probado con Claude Code ni Codex en marcha. | — | No verificado: no se lanzan agentes en las pruebas. |
| ⌘C | Enviar Ctrl-C mandaría SIGINT a los procesos del shell. | ⌘C solo copia la selección de Ghostty. | Resuelto. |
| ⌘C en Micro | ⌘C no copia la selección interna de Micro: Ghostty no sabe qué aplicación tiene el foco. | En Micro, Ctrl c, o ⇧ + arrastrar (selección de Ghostty) y ⌘C. El plugin podría resolverlo en el futuro. | **Abierto.** |
| ⌘⌫ | Ghostty envía Ctrl-U: en Micro empezaba a grabar una macro. | Ctrl-U desasignado en Micro. | Mitigado. Falta "borrar hasta el inicio de línea" en Micro. |
| ⌘P con un archivo ya abierto | Abría una segunda pestaña del mismo archivo. | Resuelto el 22-10-2026: ⌘P abre por bin/vkterm-open, que salta a la pestaña existente. | Cerrado. |
| ⌘T, ⌘D, ⌘⇧D | Pestañas y splits de Ghostty. | Modo terminal: los de Ghostty. Modo IDE: ⌘T abre una pestaña en modo terminal y ⌘D no hace nada. | Resuelto. |
| ⌘Enter en el IDE | Ghostty lo usa para la pantalla completa y lo muestra en su menú (View), que macOS resuelve antes que la tabla del IDE: el commit no llegaría. Enter no tiene una variante por tecla física sin menú, como ⌘T. | En la tabla por defecto `super+enter=ignore`: el menú se queda con ⌃⌘F (pantalla completa, como en las demás apps de macOS) y en el IDE ⌘Enter es el commit. En el modo terminal ⌘Enter no hace nada. | Resuelto (comprobación de la configuración; la tecla real, R4). |
| ⌘T, ⌘N y ⌘⇧, en el IDE | Con el atajo puesto por carácter, Ghostty lo muestra en su menú (New Tab, New Window, Reload Configuration) y macOS lo resuelve por el menú antes que por la tabla del IDE: ⌘K ⌘T abría una pestaña en vez del selector de tema, y ⌘⇧, recargaba sin devolver el modo IDE. El resto de atajos ⌘ del IDE sí llegan a la tabla (comprobado uno a uno). | En la tabla por defecto, ⌘T, ⌘N y ⌘⇧, van por tecla física (`super+key_t`, `super+key_n`, `super+shift+comma`): hacen lo mismo y el menú se queda sin atajo. En el IDE, ⌘N es «Nuevo archivo…» y ⌘⇧N abre la ventana. | Resuelto. |
| ⌃Tab / ⌃⇧Tab en el IDE | En el modo terminal son las pestañas de Ghostty (por defecto). Los eventos sintetizados solo llegan con la ventana al frente, y desde una prueba sin nadie delante macOS no deja traerla. | — | Pendiente de la prueba manual (C2). |
| ⌘⇧P | Paleta de comandos de Ghostty. | En los dos modos, la de vkTERM (en el modo terminal, tras convertir la pestaña). La de Ghostty sigue en su menú. | Aceptado. |
| Recargar la configuración | Ghostty vacía las tablas de atajos al recargar: la pestaña volvería al modo terminal con el IDE abierto, y las teclas que llegan mientras tanto (~1 s) van a la tabla de por defecto. | ⌘⇧, en el IDE y el cambio de tema o de fuente la vuelven a activar; el tamaño de letra no recarga (AppleScript `set_font_size`). Al cerrar la carpeta justo después de un cambio de tema, `vkterm-surface settle` vuelve a soltar el modo IDE a los 3 s si una reactivación tardía lo había puesto. Desde el menú de Ghostty («Reload Configuration») no se reactiva: usa ⌘⇧,. | Mitigado. |
| ⌥ Option | Con `= right`, ⌥ derecho + 2 enfocaba el editor en vez de escribir @ (en Windows y Linux la @ es AltGr + 2, el Alt derecho). | `= false`: los dos ⌥ escriben caracteres, como en VS Code. Ampliar panel pasa a ⌘K Z. | Resuelto. |
| Ctrl s/q/o/p/n/t/h/b | En modo normal, Zellij capturaba estas teclas. | Modo locked por defecto. | Resuelto. |
| Panel subido con la rueda | Zellij 0.45 pasa al modo Scroll mientras el panel con el foco no está abajo del todo (`scroll_mode_sync`). Los atajos del IDE solo están en normal y locked: al releer la salida de Claude Code, ⌘K ⌘S, ⌘1–⌘5 y los demás no hacían nada, y lo escrito lo tomaban los atajos de Scroll (e: editar el scrollback). | `scroll_mode_sync false`: se queda en locked, como el terminal de VS Code; al escribir, el panel vuelve abajo. | Resuelto (PTY, rueda por SGR). |
| Ctrl g | Zellij (desbloquear) frente a Micro (ayuda), zsh (send-break) y Claude Code (editor externo). | Gana Zellij. La ayuda de Micro está en F1. | Aceptado. |
| Alt 0–9, Alt , / ., Alt ?, Alt h/j/k/l, Alt z | Los usa vkTERM. En zsh, Alt h = run-help, Alt+dígito = argumento numérico, Alt . = última palabra. | Gana vkTERM. Desde el teclado del Mac solo se alcanzan a través de ⌘. | Aceptado. |
| ⌘K | Ghostty: limpiar pantalla. | Modo IDE: prefijo de las secuencias ⌘K ⌘S, ⌘K ⌘T, ⌘K ⌘L, ⌘K Z y ⌘K F, como en VS Code. Modo terminal: limpiar pantalla. | Resuelto. |
| ⌘+ ⌘- ⌘0 | Ghostty: tamaño de letra de la pestaña (no se recuerda). Con ellos en su menú (View), macOS resolvía ⌘- y ⌘0 por el menú antes que por la tabla del IDE (comprobado con teclas sintetizadas: ⌘+ llegaba, ⌘- no). | Sustituyen a los de Ghostty en los dos modos: el tamaño de vkTERM, en todas las ventanas, que se guarda (bin/vkterm-font). En el IDE van al plugin; en el modo terminal, F17–F19 con Super a zsh, solo en el prompt (en vim o less no hacen nada). | Resuelto. |
| ⌘F, ⌘Z, ⌘⇧Z, ⌘A, ⌘X | Acciones de Ghostty (buscar, deshacer cierres, seleccionar la pantalla) que no actuaban sobre Micro. | Modo IDE: van a Micro con el foco en el editor, como en VS Code; fuera del editor no hacen nada (la búsqueda en un terminal está en Ctrl g, s, f). Modo terminal: los de Ghostty. Las teclas con menú de macOS (⌘W, ⌘Z, ⌘A, ⌘F, ⌘C, ⌘V) llegan a la tabla y no al menú: comprobado con eventos sintetizados. | Resuelto. |
| Atajos globales | macOS no tiene atajos simbólicos sobre estas combinaciones. | — | Las pruebas G no pasan por esta capa: se confirma en la prueba manual. |
