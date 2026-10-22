# Atajos de teclado (Fase 1: interfaz tipo VS Code)

## Cómo llega una tecla a cada programa

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

## Atajos de vkTERM

| Atajo | Acción | Implementación | Verificado |
|---|---|---|---|
| ⌘B | Mostrar / ocultar el explorador (barra lateral) | Alt 6 → plugin `toggle_sidebar` | G, P, M |
| ⌘J | Mostrar / ocultar el Shell (panel inferior). Al mostrarlo recibe el foco. | Alt 7 → `toggle_panel` | G, P, M |
| ⌥⌘B | Mostrar / ocultar Claude Code y Codex (barra derecha), con cualquiera de los dos ⌥ | Alt 8 → `toggle_agents` | G, P, M |
| ⌘1 – ⌘5 | Foco en Explorador, Editor, Claude Code, Codex, Shell. Si el panel está oculto, lo muestra. | Alt 1–5 → `focus` por identidad del panel | G, P, M |
| ⌘⌥← / ⌘⌥→ | Pestaña anterior / siguiente del editor (y foco en el editor) | Alt , / Alt . → `prev_tab` / `next_tab` → Alt-, / Alt-. de Micro | G, P, M |
| ⌃⇧Tab / ⌃Tab | Igual que ⌘⌥← / ⌘⌥→ | Alt , / Alt . | G, P, M |
| ⌘W | Cerrar la pestaña del editor. Fuera del editor no hace nada (ya no cierra la ventana). | Alt 9 → `close_tab` → Ctrl-Q de Micro | G, P, M |
| ⌘P | Abrir archivo en una pestaña nueva | Alt 0 → `quick_open`: foco al editor + Ctrl-E y `tab ` en Micro (Tab completa rutas) | G, P, M |
| ⌘S | Guardar en Micro | F2 | G, P, M |
| ⌘K ⌘T | Tema de color (como VS Code): ↑ ↓ prueban cada tema, Enter lo deja, Esc vuelve al anterior. | Secuencia de Ghostty → Alt t → mensaje `toggle_themes` a otra instancia del plugin (`mode "themes"`) | G, P |
| ⌘K ⌘S | Ventana de atajos (como Unity). Se cierra con Esc, cualquier tecla, un clic, otra vez ⌘K ⌘S o al pasar el foco a otro panel. | Secuencia de Ghostty → Alt ? → mensaje `toggle_shortcuts` a otra instancia del plugin (`mode "shortcuts"`) en un panel flotante | G, P |
| ⌘⌥↑ / ⌘⌥↓ | Foco al panel de arriba / abajo (editor ↔ Shell) | Alt k / j → `MoveFocus` | G, P, M |
| ⌘Z / ⌘⇧Z | Deshacer / rehacer en el editor | Alt u / Alt y → `edit` → Ctrl-Z / Ctrl-Y a Micro, solo con el foco en el editor | P, M |
| ⌘A | Seleccionar todo en el editor | Alt a → `edit` → Ctrl-A | P, M |
| ⌘F | Buscar en el archivo | Alt s → `edit` → Ctrl-F | P, M |
| ⌘X | Cortar en el editor (sin selección, la línea) | Alt x → `edit` → Ctrl-X (Cut\|CutLine) | P (no actúa en el shell), M |
| ⌘C | Copiar: la selección de Ghostty y, en el editor, la de Micro (sin selección, la línea). Nunca envía Ctrl-C a un shell. | Acción de Ghostty encadenada con Alt c → `edit` → Ctrl-C (Copy\|CopyLine) | G (no interrumpe procesos), P, M |
| ⌘V | Pegar | Acción de Ghostty (bracketed paste) | G (Micro), P (Micro y shell), M |
| ⌘← / ⌘→ | Inicio / fin de línea | Home / End | G (Micro), P (Micro y shell), M |
| ⌘K Z | Ampliar / restaurar el panel con el foco (como el modo Zen de VS Code) | Secuencia de Ghostty → Alt z | G, P |
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
suelto sigue guardando. ⌘K solo (limpiar pantalla en Ghostty) deja de
existir.

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

⌘S o Ctrl s guardar · ⌘P abrir en pestaña nueva · Ctrl o abrir en la pestaña
actual · Ctrl f buscar · Ctrl z / Ctrl y deshacer / rehacer · ⌥ ← / → palabra
· ⇧ + flechas seleccionar · Ctrl c copiar la selección de Micro · **F1
ayuda** · Alt g menú de atajos.

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
| o | **Sesión**: d desconectar (la sesión sigue viva) · w gestor de sesiones |
| Ctrl q | Cerrar Zellij y todas las terminales de la sesión |
| Esc / Enter / Ctrl g | Volver a locked |

## Paneles de agentes

"Waiting to run": **Enter** ejecuta `claude` / `codex` · **Esc** abre un shell
en ese panel · **Ctrl c** cierra el panel.

## Conflictos encontrados

| Atajo | Conflicto | Resolución | Estado |
|---|---|---|---|
| ⌘W | En Ghostty cerraba la ventana entera. | Cierra la pestaña del editor; fuera del editor no hace nada. | Resuelto. |
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
| ⌘P con un archivo ya abierto | El comando `tab` de Micro abre una segunda pestaña del mismo archivo (mismo búfer). Desde el explorador no pasa. | — | Abierto, menor (H3: ⌘P con búsqueda). |
| ⌘T, ⌘D, ⌘⇧D | Pestañas y splits de Ghostty. | Desasignados. | Resuelto. |
| ⌥ Option | Con `= right`, ⌥ derecho + 2 enfocaba el editor en vez de escribir @ (en Windows y Linux la @ es AltGr + 2, el Alt derecho). | `= false`: los dos ⌥ escriben caracteres, como en VS Code. Ampliar panel pasa a ⌘K Z. | Resuelto. |
| Ctrl s/q/o/p/n/t/h/b | En modo normal, Zellij capturaba estas teclas. | Modo locked por defecto. | Resuelto. |
| Ctrl g | Zellij (desbloquear) frente a Micro (ayuda), zsh (send-break) y Claude Code (editor externo). | Gana Zellij. La ayuda de Micro está en F1. | Aceptado. |
| Alt 0–9, Alt , / ., Alt ?, Alt h/j/k/l, Alt z | Los usa vkTERM. En zsh, Alt h = run-help, Alt+dígito = argumento numérico, Alt . = última palabra. | Gana vkTERM. Desde el teclado del Mac solo se alcanzan a través de ⌘. | Aceptado. |
| ⌘K | Ghostty: limpiar pantalla. | Prefijo de las secuencias ⌘K ⌘S, ⌘K ⌘T y ⌘K Z, como en VS Code. | Resuelto. |
| ⌘F, ⌘Z, ⌘⇧Z, ⌘A, ⌘X | Acciones de Ghostty (buscar, deshacer cierres, seleccionar la pantalla) que no actuaban sobre Micro. | Van a Micro con el foco en el editor, como en VS Code. Fuera del editor no hacen nada (la búsqueda en un terminal está en Ctrl g, s, f). | Resuelto. |
| Atajos globales | macOS no tiene atajos simbólicos sobre estas combinaciones. | — | Las pruebas G no pasan por esta capa: se confirma en la prueba manual. |
