# Informe de la Fase 1 — Interfaz tipo VS Code

Fecha: 2026-10-21 · Equipo: Mac mini Apple M4 · Teclado: Spanish - ISO
Estado: **H0, H1 y H2 entregados, pendiente de revisión.** Plan aprobado:
[phase-1-plan.md](phase-1-plan.md).

## 1. Qué se pidió

Tras la corrección 2, el explorador (Strider) seguía abriendo terminales. La
revisión redefinió el objetivo: vkTERM debe comportarse como VS Code. La
prioridad es la barra lateral (⌘B, moverse entre archivos, abrir varios), y
después abrir y cerrar a voluntad los terminales de Claude Code, Codex y Shell.

Decisiones del usuario:

- **Layout VS Code:** explorador a la izquierda, Shell abajo, Claude Code y
  Codex a la derecha.
- **"Cerrar" = ocultar**, sin matar el proceso.
- **⌘⌥← / ⌘⌥→** para las pestañas.

## 2. Qué se entrega

| Requisito | Implementación | Evidencia |
|---|---|---|
| El explorador no abre terminales | Plugin propio `vkterm` en lugar de Strider. Enter en carpeta la pliega o despliega. | P: Enter en carpeta, número de paneles constante. |
| ⌘B muestra / oculta la barra lateral | `hide_pane_with_id` / `show_pane_with_id` + `override_layout` del estado visible | P y G: el editor ocupa el espacio; al volver, geometría idéntica. |
| Moverse entre archivos | Árbol con ↑ ↓ ← →, Enter, Espacio, salto por letra, clic y rueda | P y G (teclado); P (clic). |
| Abrir varios archivos | Cada archivo, una pestaña de Micro. Si ya está abierto, salta a su pestaña. Pestañas con ⌘⌥← →, ⌃Tab, clic. ⌘W cierra la pestaña. | P y G. |
| Abrir / cerrar Claude Code, Codex y Shell | ⌘J (Shell), ⌥⌘B (agentes), ⌘3–⌘5 (los muestran si están ocultos) | P y G: los procesos no cambian; geometría idéntica tras cualquier orden. |
| Layout VS Code | Explorador 20 % · editor 55 % (Shell abajo, 30 %) · agentes 25 % | P y G. |

También se corrigió:

- **⌘W** ya no cierra la ventana de Ghostty: fuera del editor no hace nada.
- **Al cerrar la última pestaña**, el editor vuelve a abrirse vacío en vez de
  quedar "EXITED".
- **Binarios:** no se abren y se avisa en el explorador.
- **⌘1–⌘5 enfocan por identidad del panel**, no por posición.
- **Sin panel temporal:** el explorador abre los archivos en segundo plano.

Se hicieron capturas de la ventana real de Ghostty (teclas sintetizadas) en cinco estados:
inicio, árbol con dos pestañas, sin barra lateral, solo editor, todo de vuelta. Se
adjuntan a la entrega; no se versionan.

## 3. H0: prueba técnica

| Comprobación | Resultado |
|---|---|
| Ocultar / mostrar explorador, Shell y agentes en tres órdenes distintos | Geometría final idéntica a la inicial |
| PIDs de Micro, Shell y paneles en espera tras ocultar / mostrar | Sin cambios |
| `MessagePlugin` desde un atajo en LOCKED, con el plugin oculto | Llega (`PipeSource::Keybind`) |
| `run_command` de `vkterm-open` desde el plugin con `ZELLIJ_SESSION_NAME` | Abre la pestaña en Micro |
| Ruta del plugin | `file:/$VKTERM_HOME/…`: Zellij expande variables; no hace falta generar la configuración |
| Permisos del plugin | Zellij los pide una vez en el panel; las pruebas responden `y` |
| Códigos Alt 6–9, Alt , / Alt . | Zellij los acepta en KDL y llegan desde Ghostty |

## 4. Pruebas automatizadas

**Estado tras los añadidos de la noche del 21 al 22-10** (secciones 10 a 12):

| Ejecución | Código | Resultado |
|---|---|---|
| `scripts/smoke-test.py`, dos veces seguidas | Final | **129/129** las dos: configuración 10 (`cargo test`: 22), PTY 118, sesiones 1 |
| `--ghostty` completa | Con la ventana de atajos, antes del teclado y los temas (23:14) | 157/157 |
| Solo la parte de Ghostty | Con el teclado nuevo (23:18) | 39/39 |
| Ghostty con temas y atajos de edición | — | **Sin ejecutar**: la pantalla del Mac estaba bloqueada y macOS no crea ventanas así |

Antes de la revisión hay que ejecutar `scripts/smoke-test.py --ghostty` con
la pantalla desbloqueada.

Entrega inicial y atenuado (sección 9), con `scripts/smoke-test.py --ghostty`
en dos ejecuciones consecutivas:

| Modo | Qué hace | Resultado |
|---|---|---|
| Configuración | `cargo test` del plugin (14 tests entonces), `zellij setup --check`, configuración y atajos efectivos de Ghostty | 10/10 |
| PTY | `bin/vkterm` en un pseudo-terminal 200×50, con los bytes que emite Ghostty | 94/94 |
| `--ghostty` | Ghostty real, eventos CGEvent entregados a su proceso (**no son pulsaciones físicas**) | 41/41 |
| Sesiones previas | Las sesiones que existían siguen vivas | 1/1 |

**Total: 146/146 en las dos ejecuciones** (tras añadir el atenuado, sección 9;
la entrega inicial dio 137/137).

Garantías: nunca ejecuta Claude Code ni Codex; no modifica el portapapeles (en
la prueba de ⌘V se pega en una pestaña vacía que se cierra sin guardar); no
toca las sesiones existentes.

## 5. Cambios respecto al plan

- **H1 y H2 se entregan juntos.** El controlador de zonas es el mismo código
  para ⌘B, ⌘J y ⌥⌘B. Además, el nuevo layout obligaba a hacer ⌘2–⌘5 por
  identidad ya en H1: las macros posicionales dejaban de servir.
- **No hace falta generar la configuración** (punto 6 del H0): basta con
  `$VKTERM_HOME` en la ruta del plugin.
- **El plugin abre los archivos con `bin/vkterm-open`.** Leer la pantalla de
  Micro desde el propio plugin (`get_pane_scrollback`) habría exigido un
  permiso más.
- **⌘⌥↑ / ⌘⌥↓ siguen moviendo el foco** entre editor y Shell; el plan no los
  mencionaba.
- **⌘W en la última pestaña** deja el editor vacío (estaba en H3); era
  necesario para que ⌘W no dejara un panel "EXITED".

## 6. Pendiente y limitaciones conocidas

- **Pulsaciones físicas** y atajos globales de macOS:
  [manual-tests.md](manual-tests.md).
- **Primer clic en el explorador sin foco:** Zellij lo usa solo para
  enfocarlo; el segundo actúa.
- **Tamaños redimensionados a mano** se pierden al ocultar o mostrar (H3).
- **⌘P con un archivo ya abierto** abre una segunda pestaña del mismo archivo
  (mismo búfer). ⌘P con búsqueda difusa: H3.
- **⌘C sobre la selección interna de Micro** sigue abierto.
- **Sin probar con Claude Code o Codex en marcha:** ⌘S, ⌘P y ⌘W. Ocultar y
  mostrar sí se probó con el panel Codex convertido en shell (Esc).
- **Sin prueba automática:** rueda, Re Pág / Av Pág, Ctrl h y refresco
  automático del árbol.

## 7. Problemas técnicos encontrados

1. **`show_pane_with_id` no devuelve el panel a su sitio** y las swap layouts
   no distinguen qué zona falta. Solución: `override_layout` con el layout
   exacto, emparejando por comando. El Shell va envuelto en `bin/vkterm-shell`
   para tener un comando propio.
2. **`dump-screen` devuelve filas antiguas** cuando un panel se encoge. Las
   pruebas y `vkterm-open` buscan la barra de estado de Micro de arriba abajo.
3. **Micro ofrece recuperar copias de seguridad** de archivos que quedaron sin
   guardar en sesiones anteriores, lo que interrumpe `vkterm-open`. Solo pasó
   con un proyecto de prueba reutilizado; la suite usa carpetas nuevas.
4. **`zellij action pipe` desde la CLI se queda esperando** si el plugin no la
   libera, y liberarla requiere otro permiso. Las pruebas usan atajos.
5. **El primer clic solo enfoca** el panel (Zellij).
6. **El portapapeles real del usuario** desplazaba la vista de Micro en la
   prueba de ⌘V. Ahora se pega en una pestaña vacía.

## 8. Siguiente paso recomendado

1. Revisión con el teclado físico (`docs/manual-tests.md`, secciones B, C y D
   primero).
2. H3, según prioridades del usuario:
   - ⌘P con búsqueda difusa (vista flotante del mismo plugin);
   - conservar tamaños redimensionados;
   - crear, renombrar y borrar desde el explorador;
   - colores de git;
   - ⌘C contextual en Micro.

## 9. Añadido tras la entrega: paneles sin foco atenuados

Petición del usuario: como en iTerm2, que el panel con el foco se vea bien y
los demás queden atenuados.

**Qué hace.** El panel con el foco se ve con sus colores normales. Los demás,
explorador incluido, tienen el fondo más oscuro (crust) y el texto sin color
apagado (overlay0). Los colores están en `config/zellij/config.kdl`
(`inactive_fg`, `inactive_bg`).

**Cómo.**

- Zellij 0.45.1 no tiene esta opción ni puede atenuar el contenido de un panel.
  El plugin cambia el color por defecto de texto y fondo de cada terminal
  (`set_pane_color`), como `window-style` en tmux.
- El explorador sin foco se dibuja con esos mismos colores (SGR de 24 bits).
- No pide permisos nuevos.

**Problemas encontrados y resueltos.**

1. **Zellij reinicia los colores** de un panel en espera ("Waiting to run")
   al redimensionarlo, y al arrancar o terminar su comando. Tras ⌘B, Codex se
   quedaba sin atenuar. El plugin ya no recuerda lo que envió: compara con los
   colores que Zellij informa de cada panel (`PaneInfo.default_fg/bg`).
2. **"SCROLL: 0/18" en el marco del explorador** tras ⌘B. Las líneas con fondo
   de color no se recortan al estrechar el panel. Zellij las pasa al scroll y,
   aunque el render vacía el panel, no recalcula el contador. El plugin envía
   `CSI 3 J` en cada render. Se comprobó que la prueba nueva falla si se quita
   esa línea.

**Pruebas nuevas (PTY, 9).**

- Con el foco en cada panel (⌘1–⌘5), solo ese panel conserva sus colores.
- El explorador usa el mismo fondo sin foco y los colores normales con foco.
- Tras ⌘B, los paneles redimensionados siguen atenuados.
- El marco no muestra `SCROLL`.

Capturas de Ghostty con el foco en cada panel: se adjuntan a la entrega.

**Prueba de ⌘V corregida.** Con el código del atenuado, dos de las tres
primeras ejecuciones completas fallaron en la parte de Ghostty. El primer
síntoma era `Header.tsx` modificado por una sola letra. La prueba de ⌘V pega
el portapapeles real en una pestaña vacía y la cierra con Ctrl-Q y `n`. Si el
pegado no llega, Ctrl-Q cierra sin preguntar y la `n` se escribe en
`Header.tsx`. Ahora la prueba confirma que el pegado llegó y solo envía `n` si
Micro pregunta.

- Después: cinco ejecuciones de la parte de Ghostty y dos completas, todas sin
  fallos.
- No se pudo reproducir por qué no llegó el pegado. El portapapeles cambia
  entre ejecuciones.
- Los fallos de pestañas y ⌘B de esas dos ejecuciones no se han vuelto a ver.

**Limitaciones.**

- El resaltado de sintaxis, los prompts y las barras de Micro conservan sus
  colores sin foco: Zellij solo cambia el texto y el fondo sin color propio.
- Si un programa reinicia el terminal (`reset`) en un panel sin foco, ese
  panel pierde el atenuado hasta que cambie algo en los paneles (foco, tamaño
  o título).

## 10. Añadido tras la entrega: ventana de atajos (⌘K ⌘S)

Petición del usuario: un modal con los atajos, como el de Unity (Ubuntu).

**Cómo lo hace Unity 7.** Lo estudié en su código fuente (`unity/shortcuts`,
copia de Ubuntu en git.launchpad.net):

- **Disparo:** mantener Super. Aparece a los 650 ms (`SUPER_TAP_DURATION`)
  con un fundido de 100 ms, centrado en el monitor del ratón, y se oculta al
  soltar.
- **Forma:** título "Keyboard Shortcuts" en negrita, una línea separadora y
  las categorías en columnas de tres (`categories_per_column = 3`).
- **Filas:** el atajo en negrita a la izquierda y la descripción a la
  derecha, con separadores entre categorías.
- **Teclas:** se leen de la configuración activa de Compiz.

**Qué hace vkTERM.**

- **Atajo:** ⌘K ⌘S (decisión del usuario, como en VS Code). Es una
  secuencia: ⌘K y después ⌘S.
- **Ventana:** flotante y centrada, con el título "Atajos de teclado" y seis
  categorías en dos columnas: Paneles, Editor, Explorador · Agentes y Shell,
  Zellij, Teclado. Atajos en naranja, categorías en verde y el resto de
  paneles atenuado.
- **Ventana estrecha:** si no caben las dos columnas, pasa a una sola con
  desplazamiento.
- **Cierre:** Esc, cualquier tecla, un clic, otra vez ⌘K ⌘S o mover el foco
  a otro panel.
- **Sin permisos nuevos.**

**Problemas encontrados y resueltos.**

1. `is_focused` es por capa. Con la ventana abierta, el editor seguía
   "enfocado" y no se atenuaba; tras ⌘5, la ventana no se cerraba. Ahora se
   usa la visibilidad de la capa flotante.
2. **Permisos.** Al pedir la ventana solo dos permisos, Zellij sobrescribió
   la caché y el explorador volvía a preguntar en la siguiente sesión. Lo
   destapó una captura de Ghostty: las pruebas PTY conceden los permisos solas
   y lo ocultaban. Ahora las dos instancias piden los mismos cuatro.
3. **Marco de Zellij.** Añadía "PIN [ ]" y la ayuda para redimensionar. La
   ventana va sin marco y dibuja el suyo.

**Pruebas nuevas.**

- **PTY (10):**
  - Abre flotante y con el foco, centrada (x=43, y=7, 113×34 en 200×49).
  - Dos columnas de tres categorías.
  - Incluye todos los atajos ⌘ del perfil de Ghostty.
  - Atenúa el resto de paneles.
  - Se cierra con ⌘K ⌘S, con Esc y al pasar el foco con ⌘5.
  - No deja paneles de más.
  - La caché conserva los cuatro permisos del explorador. Se comprobó que
    esta prueba falla si la ventana vuelve a pedir menos.
- **Ghostty (1):** ⌘K ⌘S abre la ventana y Esc la cierra.
- **cargo test (5):** columnas, una sola columna, todos los atajos, alineación
  y separadores.

**Limitaciones.**

- No se abre manteniendo ⌘ como en Unity: Ghostty no detecta ⌘ pulsado solo.
- La lista es una tabla del plugin, no se lee de la configuración. La prueba
  automática solo comprueba que están los atajos ⌘.
- Tras ⌘K, Ghostty espera la segunda tecla sin límite de tiempo.

## 11. Añadido: teclado normal, como en VS Code

Petición del usuario: "quiero usar mi teclado normal, como hacer la arroba con
Alt + 2".

- **Causa:** el ⌥ derecho actuaba como Alt, así que ⌥ derecho + 2 enfocaba el
  editor (Alt 2) en vez de escribir @. En Windows y Linux, la @ del teclado
  español es AltGr + 2, y AltGr es el Alt derecho.
- **Cambio:** `macos-option-as-alt = false`. Los dos ⌥ escriben los
  caracteres del teclado, como en cualquier app de macOS y en VS Code.
  ⌥← / ⌥→, que no producen carácter, siguen saltando palabra.
- **Lo que hacía el ⌥ derecho tiene atajo con ⌘:**
  - ampliar el panel pasa a **⌘K Z** (modo Zen de VS Code);
  - pestañas y foco ya tenían ⌘⌥← → y ⌘1–⌘5.
- **Pruebas (Ghostty):** ⌥2 y ⌥3 escriben `@@#` con los dos ⌥; ⌘K Z amplía y
  restaura; ⌥→ / ⌥← saltan palabra con los dos ⌥.

**Atajos de edición de VS Code**, que Ghostty usaba para otras cosas:

- **Atajos:** ⌘Z / ⌘⇧Z deshacer / rehacer, ⌘A seleccionar todo, ⌘F buscar,
  ⌘X cortar y ⌘C copiar la selección de Micro. Sin selección, ⌘X y ⌘C actúan
  sobre la línea entera, como VS Code.
- **Ruta:** van a Micro a través del plugin, solo con el foco en el editor. En
  un shell, Ctrl-Z suspendería el proceso y Ctrl-C lo interrumpiría.
- **⌘C** sigue copiando además la selección de Ghostty (acción encadenada).
- **Pruebas PTY:** deshacer / rehacer, buscar y seleccionar todo en Micro.
  ⌘Z, ⌘C y ⌘X en el Shell no suspenden ni interrumpen un `sleep`.
- **Sin prueba automática:** ⌘C y ⌘X, porque tocarían el portapapeles del
  usuario. Están en la lista manual (G2c, G2d).

## 12. Añadido: temas (⌘K ⌘T)

Como "Preferences: Color Theme" de VS Code: al moverse por la lista se prueba
cada tema, Enter lo deja y Esc vuelve al anterior. El tema cambia a la vez en
Ghostty, Zellij, Micro y el atenuado. Detalle técnico en
[architecture.md](architecture.md#temas-k-t).

- **Nueve temas:**
  - Catppuccin Mocha (predeterminado) y Latte;
  - Dark Modern y Light Modern de VS Code;
  - Dracula, Nord, Tokyo Night, y Gruvbox oscuro y claro.
- **Estado:** en `config/local/`, dentro del proyecto y fuera de git.
  `VKTERM_STATE_DIR` lo cambia de sitio; las pruebas lo usan para no tocar el
  tema del usuario.
- **Micro** guarda ahora allí su estado (`config/local/micro`), en lugar de en
  `config/micro`, que está versionado.
- **Permisos:** dos nuevos (Reconfigure y MessageAndLaunchOtherPlugins). El
  explorador los pide una vez.

**Problemas encontrados y resueltos:**

- `$options` es una variable especial de zsh. `bin/vkterm-editor` la usaba
  para los argumentos de Micro, que recibía como archivos "on" y "off". Lo
  detectó la suite PTY.
- `vkterm-theme value` devolvía error si aún no había tema, y `bin/vkterm`,
  que usa `err_exit`, se habría cortado al arrancar.

**Pruebas PTY nuevas (10):**

- Abre el selector y lista los nueve temas.
- ↓ prueba un tema: se guarda, cambian Zellij y el atenuado.
- Esc vuelve al anterior.
- Enter lo deja.
- Micro pasa al esquema claro.
- Una sesión nueva arranca con el tema guardado.
- Se vuelve al predeterminado.

**Sin verificar en Ghostty real:** la pantalla del Mac estaba bloqueada y
macOS no crea ventanas así. Quedan pendientes el cambio en vivo del propio
Ghostty (`SIGUSR2`), sus capturas y la prueba `--ghostty` de ⌘K ⌘T.

