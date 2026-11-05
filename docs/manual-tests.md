# Pruebas manuales con teclado físico

Las pruebas automáticas (`scripts/smoke-test.py --ghostty`) usan eventos de
teclado sintetizados entregados al proceso de vkTERM Test.app. Esta lista
confirma lo mismo con el **teclado físico del Mac**, incluida la capa de
atajos globales de macOS, que las pruebas automáticas no atraviesan.

## Preparación

```sh
cd /Volumes/Apps/desarrollo/vkTERM
bin/vkterm-app install --link      # ~/Applications/vkTERM.app y el comando vkterm
```

Abre **vkTERM** desde ~/Applications, Spotlight o el Dock. La primera vez,
macOS pide permiso para que vkTERM acceda al disco externo (/Volumes/Apps):
pulsa **Permitir**. En la pestaña que se abre:

```sh
cd /Volumes/Apps/desarrollo/vkTERM/examples/demo && vkterm .
```

La pestaña pasa a ser el IDE de demo. La primera vez, el arranque tarda unos
segundos más porque se compila el plugin.

El explorador **no** debe preguntar `Allow? (y/n)`: vkTERM guarda los
permisos del plugin en la caché de Zellij (`bin/vkterm-permissions`). Si lo
pregunta, anótalo como fallo, pulsa **y** en ese panel y sigue.

Anota ✔ / ✘ en cada fila. Si algo falla, anota también qué ves en la barra
de estado (abajo; tras Ctrl g muestra NORMAL) y en qué panel estaba el foco:
es el panel con el marco resaltado.

## A. Layout

| # | Acción | Resultado esperado |
|---|---|---|
| A1 | Mirar la ventana | Tema oscuro. Abajo, la barra de estado (`⌨ vkTERM` con fondo de color y `demo` a la izquierda, `⌘K F cerrar carpeta · ⌘K ⌘S atajos` a la derecha). Con vkTERM.app creada desde `scripts/build-ghostty.sh`, ningún botón flotante «⌨ vkterm» de Ghostty; con ⌘K, mientras espera la segunda tecla, sí aparece abajo «⌘K …». |
| A2 | Mirar el layout | Explorador a la izquierda con `DEMO`, `▸ src` y `.vkterm`. Editor en el centro con `Header.tsx`, Shell debajo del editor, Claude Code y Codex a la derecha, uno sobre otro. |
| A3 | Mirar Claude Code y Codex | "Waiting to run: claude" / "codex". Ningún agente arrancado. |
| A4 | Mirar la barra de estado | No muestra ningún modo (está en LOCKED). En un proyecto con git, muestra la rama (`⎇ main`). |
| A5 | ⌘1, ⌘2, ⌘3, ⌘4, ⌘5 | El panel con el foco se ve con sus colores normales. Los demás, explorador incluido, tienen el fondo más oscuro y el texto sin color apagado. El resaltado de sintaxis y los prompts mantienen sus colores. |
| A6 | ⌘3, ⌘B dos veces, ⌘2 | Codex sigue atenuado. El título del explorador no muestra `SCROLL`. |

## B. Barra lateral y explorador (prioridad)

| # | Acción | Resultado esperado |
|---|---|---|
| B1 | ⌘B | El explorador desaparece y el editor y el Shell ocupan su espacio. |
| B2 | ⌘B | El explorador vuelve, con el mismo ancho y el mismo estado. |
| B3 | ⌘1, → | Se despliega `src` (`▾ src`, `▸ components`, `App.tsx`). |
| B4 | → , → , → | Baja a `components`, la despliega y baja a `Header.tsx`. |
| B5 | ↓ y Enter (en `App.tsx`) | App.tsx se abre en una **pestaña nueva** del editor; arriba se ven las dos pestañas. El foco pasa al editor. No aparece ningún panel nuevo. |
| B6 | ⌘1, ↑ y Espacio (en `Header.tsx`) | Salta a la pestaña de Header.tsx, sin abrir otra. El foco **sigue en el explorador**. |
| B7 | Enter sobre una carpeta, varias veces | La pliega y despliega. **Nunca se abre un terminal.** |
| B8 | ← varias veces | Pliega y sube hasta la raíz; no sale del proyecto. |
| B9 | Escribir `a` | Salta a la siguiente entrada que empieza por "a". |
| B10 | Con el explorador enfocado: clic en una carpeta y en un archivo | La carpeta se pliega / despliega; el archivo se abre. (Sin foco, el primer clic solo enfoca el explorador.) |
| B11 | Rueda del ratón sobre el explorador con `src` desplegado | La lista se desplaza. |
| B12 | En Shell: `touch src/nuevo.ts` | Aparece en el explorador sin hacer nada más. |
| B13 | Ctrl h en el explorador | En un proyecto con `.git`, aparece y desaparece. |

## C. Pestañas del editor

| # | Acción | Resultado esperado |
|---|---|---|
| C1 | ⌘⌥← y ⌘⌥→ | Pestaña anterior / siguiente. |
| C2 | ⌃Tab y ⌃⇧Tab | Pestaña siguiente / anterior. |
| C3 | Clic en el nombre de una pestaña | Cambia a esa pestaña. |
| C4 | ⌘W con el foco en el editor | Cierra la pestaña activa. |
| C5 | ⌘W con el foco en el Shell | No pasa nada: **no se cierra la ventana**. |
| C6 | ⌘W hasta cerrar la última pestaña | El editor queda vacío (`No name`) y sigue disponible. |
| C7 | ⌘P, escribir `app` y Enter | Ventana «Abrir archivo» centrada; `src/App.tsx` queda el primero; se abre en una pestaña. Repetir con el archivo ya abierto salta a su pestaña. |
| C8 | ⌘⇧P, escribir `tema` y Enter | Ventana «Paleta de comandos»; «Preferencias: Tema de color» abre el selector de tema. ⌘⇧P dos veces la cierra; ⌘P sobre ella cambia a archivos. |
| C10 | En el explorador, seleccionar `src` y pulsar **⌘N**; escribir `App.tsx`, luego borrar y escribir `ui/Nuevo.tsx`, Enter | Ventana «Nuevo archivo» con «En src/»; con `App.tsx` avisa «Ya existe src/App.tsx»; con Enter, `src/ui/Nuevo.tsx` se crea vacío, aparece seleccionado en el explorador y se abre en una pestaña. **No se abre una ventana nueva de vkTERM: si se abre, el menú de Ghostty se ha quedado ⌘N; anótalo.** |
| C11 | **⌘⇧N** en el IDE | Se abre una ventana nueva de vkTERM, en modo terminal. |
| C9 | Cambiar un archivo sin guardar, abrir otro con ⌘P, escribir algo, pulsar **Ctrl q** (no ⌘W) y responder `n` | La pestaña se cierra sin guardar y la `n` **no** aparece en el otro archivo. Con teclas sintetizadas Ghostty a veces duplica la tecla que sigue a Ctrl q; falta saber si pasa con el teclado físico. |

## D. Terminales: mostrar y ocultar

| # | Acción | Resultado esperado |
|---|---|---|
| D1 | ⌘J | El Shell desaparece y el editor ocupa la columna. |
| D2 | ⌘J | El Shell vuelve, con el foco. |
| D3 | ⌥⌘B con el ⌥ izquierdo, y otra vez con el derecho | Claude Code y Codex desaparecen y vuelven. |
| D4 | Arrancar Claude Code (⌘3, Enter), escribir algo, ⌥⌘B dos veces | La conversación sigue igual: ocultar no cierra el proceso. |
| D5 | ⌘B, ⌘J, ⌥⌘B | Solo queda el editor, a pantalla completa. |
| D6 | ⌘5, ⌘3, ⌘1 | Cada uno vuelve a mostrar su panel y le da el foco. |
| D7 | En Shell: `sleep 100`, Enter, ⌘J, ⌘J | `sleep` sigue en marcha. |
| D8 | Captura al portapapeles (⌃⇧⌘4), ⌘3 y ⌘V en Claude Code; repetir en Codex (⌘4) | El agente adjunta la imagen (`[Image #1]` en Claude Code). Con texto en el portapapeles, ⌘V pega solo el texto. |

## E. Foco (⌘1–⌘5)

| # | Pulsar | Foco esperado |
|---|---|---|
| E1 | ⌘1 | Explorador |
| E2 | ⌘2 | Editor |
| E3 | ⌘3 | Claude Code |
| E4 | ⌘4 | Codex |
| E5 | ⌘5 | Shell |
| E6 | ⌘⌥↓ desde el editor y ⌘⌥↑ desde el Shell | Shell / editor |

## F. ⌥ Option

| # | Acción | Resultado esperado |
|---|---|---|
| F1 | En Shell: ⌥2 y ⌥3, primero con el ⌥ izquierdo y luego con el derecho | Escribe `@#@#`. El foco no cambia. |
| F2 | En Shell: las combinaciones con ⌥ para `[ ] { } \| \ ~`, con los dos ⌥ | Escribe esos caracteres. |
| F3 | En el editor: ⌘K Z dos veces | Amplía el editor y lo restaura. |
| F4 | En el editor: ⌥← / ⌥→ con los dos ⌥ | Salta palabra a palabra. |

## G. Guardar, copiar, pegar

| # | Acción | Resultado esperado |
|---|---|---|
| G1 | ⌘2, escribir `// prueba`, ⌘S | La barra de Micro dice `Saved …`. La barra de estado sigue sin mostrar ningún modo. |
| G2 | Copiar un texto en otra app y ⌘V en el editor | Se pega en Micro. |
| G2b | En el editor: escribir algo, ⌘Z, ⌘⇧Z | Se deshace y se rehace. |
| G2c | En el editor: seleccionar una palabra con ⇧→, ⌘C, ir a otra línea, ⌘V | Se pega la palabra. Sin selección, ⌘C copia la línea entera. |
| G2d | En el editor: ⌘X sin selección | Corta la línea entera. |
| G2e | En el editor: ⌘A y luego ⌘F | Selecciona todo; ⌘F abre "Find:" abajo (Esc lo cierra). |
| G2f | En Shell: `sleep 30`, Enter, ⌘Z y ⌘X | `sleep` sigue en marcha: no se suspende. |
| G2g | Con el foco en el editor (⌘2), arrastrar con el ratón sobre una palabra del Shell y soltar; ir a otra app y pegar | El mismo arrastre enfoca el Shell y selecciona; al soltar se copia: se pega esa palabra. |
| G3 | ⌘V en Shell con un texto de varias líneas | Se pega en el prompt **sin ejecutarse**. |
| G4 | En Shell: `sleep 30`, Enter, ⌘C | `sleep` **no** se interrumpe. |
| G5 | En Shell: `for i in $(seq 20); do echo $i; sleep 0.3; done` y ⌘S a mitad | La salida **no** se congela (puede aparecer `^[OQ`). |

## H. Sesiones

| # | Acción | Resultado esperado |
|---|---|---|
| H1 | Ctrl g | La barra de estado muestra `NORMAL`. Esc lo quita (vuelve a LOCKED). |
| H2 | Cerrar la ventana del IDE con el botón rojo; Ghostty pregunta «Close Window?»: Close. Después, `zellij list-sessions` en otro terminal | La sesión de demo ya no existe; tus otras sesiones siguen. |
| H3 | Ctrl g, o, d | No desconecta: la sesión dura lo que su pestaña. |

## I. Ventana de atajos (⌘K ⌘S)

⌘K ⌘S es una secuencia: ⌘K y después ⌘S (se puede mantener ⌘ pulsado).

| # | Acción | Resultado esperado |
|---|---|---|
| I1 | ⌘2, ⌘K ⌘S | Ventana centrada "Atajos de teclado" con dos columnas de categorías (Paneles, Editor, Explorador · Agentes y Shell, Ventana, Teclado). El resto de paneles se atenúa. |
| I2 | ⌘K ⌘S | Se cierra y el foco vuelve al editor. |
| I3 | ⌘K ⌘S, Esc (o cualquier letra) | Se cierra. La tecla no llega al editor. |
| I4 | ⌘K ⌘S, ⌘5 | Se cierra y el foco pasa al Shell. |
| I5 | ⌘K ⌘S, clic dentro o fuera de la ventana | Se cierra. |
| I6 | ⌘S solo, en el editor | Guarda, como siempre. |
| I7 | Con la ventana de Ghostty estrecha (la mitad de la pantalla), ⌘K ⌘S | Una sola columna; ↑ ↓ desplazan y cualquier otra tecla cierra. |
| I8 | ⌘K F y `vkterm .` otra vez | El explorador **no** vuelve a pedir permisos. |
| I9 | Con un tema distinto (⌘K ⌘T, p. ej. Kimbie Dark), ⌘K ⌘S | La ventana tiene el fondo y el texto del editor de ese tema y cada atajo va en una etiqueta con fondo propio, como en VS Code. |

## J. Temas (⌘K ⌘T)

| # | Acción | Resultado esperado |
|---|---|---|
| J1 | ⌘K ⌘T | Ventana "Tema de color" con los 24 temas agrupados (temas claros, oscuros y de alto contraste); ● marca el actual (Catppuccin Mocha). El resto se atenúa. |
| J2 | Escribir `dark+` | Queda Dark+ y se prueba: fondo #1e1e1e, barra lateral gris (#252526), barra de estado azul (#007acc), palabras clave moradas y cadenas naranjas en el editor. |
| J3 | Borrar y escribir `light+` | Light+: fondo blanco, palabras clave moradas (#af00db), cadenas rojas (#a31515). |
| J4 | Esc | Vuelve todo a Catppuccin Mocha. |
| J5 | ⌘K ⌘T, recorrer la lista con ↓ deprisa y Enter | Se queda el último. El editor no recibe ninguna tecla (el archivo no aparece modificado). |
| J6 | ⌘K F y `vkterm .` otra vez | El IDE vuelve a abrirse con ese tema (Ghostty, Zellij, Micro y la barra de estado). |
| J7 | Elegir Dark High Contrast | Bordes cian, foco naranja; los paneles sin foco **no** se atenúan. |
| J8 | Tras cambiar el tema, ⌘B | El explorador se oculta: la pestaña sigue en modo IDE. |
| J9 | ⌘K ⌘T, escribir `mocha`, Enter | Vuelve a Catppuccin Mocha. |

## N. Fuente (⌘K ⌘L) y tamaño (⌘+ ⌘- ⌘0)

| # | Acción | Resultado esperado |
|---|---|---|
| N1 | ⌘K ⌘L | Ventana "Fuente" con «(predeterminada)» y tus fuentes monoespaciadas (JetBrains Mono, Hack y Meslo Nerd Font, Menlo…). |
| N2 | ↓ varias veces; → dos veces | Cada fuente se prueba en toda la ventana; → agranda la letra. |
| N3 | Esc | Vuelven la fuente y el tamaño de antes. |
| N4 | En el IDE, ⌘+ (la tecla + del teclado español) dos veces y ⌘- una | La letra crece dos pasos y baja uno, en todas las ventanas de vkTERM. **Si en vez de eso solo cambia esta pestaña, el menú de Ghostty se ha quedado el atajo: anótalo.** El menú View de vkTERM ya no muestra atajos junto a Increase/Decrease/Reset Font Size. |
| N5 | Cerrar vkTERM (⌘Q) y abrirla | Se mantiene el tamaño de N4. |
| N6 | ⌘0 | Vuelve a 13 pt. |
| N7 | En el prompt de una pestaña en modo terminal, ⌘+ y ⌘0 | Igual que en el IDE: todas las ventanas, y se recuerda. Dentro de vim o less, ⌘+ no hace nada. |

## O. Brackets

| # | Acción | Resultado esperado |
|---|---|---|
| O1 | En el editor, escribir `foo(` | Aparece `foo()` con el cursor dentro. |
| O2 | Seleccionar una palabra (⇧←) y escribir `(` (o `"`, `[`, `{`) | La palabra queda envuelta: `(palabra)`. ⌘Z lo deshace de una vez. |
| O3 | Poner el cursor junto a un `{` | Su pareja `}` se resalta con el color del tema. |
| O4 | Abrir un archivo con sangría (Header.tsx) | Rayas finas `│` en cada nivel de sangría. |

## Q. Menú contextual (clic derecho en el editor; ⌘. en el explorador)

| # | Acción | Resultado esperado |
|---|---|---|
| Q1 | ⌘2 y clic derecho sobre una palabra | Junto al puntero, un menú con Cortar ⌘X, Copiar ⌘C, Pegar ⌘V, Seleccionar todo ⌘A y Paleta de comandos… ⌘⇧P. El cursor va adonde hiciste clic y el editor no se atenúa. **Si en su lugar sale el menú de Ghostty (Copy, Paste, Split…), anótalo.** |
| Q2 | Seleccionar una palabra con ⇧→, clic derecho sobre ella, Copiar; clic derecho en otra línea, Pegar | La selección sigue al abrir el menú; la palabra se pega donde hiciste el segundo clic. |
| Q3 | Clic derecho sin selección, Cortar | Corta la línea entera, como ⌘X. |
| Q4 | Clic derecho y pasar el ratón por las entradas; luego ↑ ↓ y Esc | Se resalta la entrada bajo el ratón; las flechas se saltan las rayas; Esc cierra y el foco vuelve al editor. |
| Q5 | Clic derecho, clic fuera del menú, y otra vez clic derecho en otro sitio | El clic fuera lo cierra; el segundo clic derecho vuelve a abrirlo, en su sitio. |
| Q6 | Clic derecho cerca del borde derecho y del de abajo del editor; luego «Paleta de comandos…» | El menú se abre hacia la izquierda o hacia arriba, sin salirse. La entrada abre la paleta de comandos. |
| Q7 | ⌘1, seleccionar `src` y pulsar **⌘.**; ↓ ↓ Enter | Debajo de `src`, el menú del explorador (Nuevo archivo… ⌘N, Renombrar… ⌘R, Mostrar en Finder…). Renombrar… pide el nombre nuevo de `src`; Esc cancela. **Si no pasa nada, o macOS emite un pitido, anótalo.** |
| Q8 | ⌘2 y **⌘.**; luego Esc. Después ⌘1 y **⇧F10** (con fn si las F son de sistema) | ⌘. desde el editor pasa el foco al explorador y abre el menú; Esc lo cierra y el foco queda en el explorador. ⇧F10 también lo abre. El clic derecho en el explorador no hace nada (Zellij 0.45.1). |

## K. vkTERM.app y modo terminal

| # | Acción | Resultado esperado |
|---|---|---|
| K1 | Abrir vkTERM desde ~/Applications | Una ventana con tu shell de siempre (oh-my-zsh, powerlevel10k) en `~`. El Dock y ⌘Tab muestran el icono y el nombre de vkTERM. |
| K2 | ⌘T, ⌘D, ⌘W, ⌘W | Pestaña nueva, división y cierre, como en Ghostty. |
| K3 | `ls`, ⌘F, escribir `bin`, Esc; después ⌘K | ⌘F busca en la pantalla; ⌘K la limpia. |
| K4 | En `~`, ⌘B | No se convierte: abajo aparece «vkTERM: entra antes en la carpeta de un proyecto…». |
| K5 | `cd …/examples/demo`, escribir `echo hola` **sin** Enter, ⌘P | La pestaña pasa al IDE de demo con la ventana «Abrir archivo». |
| K6 | Esc, ⌘K F | Vuelve el shell de antes, con su scroll, y `echo hola` en el prompt. |
| K7 | ⌘J | La pestaña vuelve a ser el IDE, con el foco en el Shell. |
| K8 | ⌘T, y en la pestaña nueva ⌘B | ⌘T abre una pestaña en modo terminal en la carpeta de demo. ⌘B ahí trae el IDE de demo a la pestaña nueva (la carpeta ya estaba abierta); la de antes vuelve a su shell. |
| K9 | En iTerm (u otro terminal): `vkterm /ruta/a/otro/proyecto` | vkTERM pasa al frente con una ventana nueva ya convertida en ese proyecto. |
| K10 | Repetir K9 con la misma carpeta | Enfoca esa ventana; no abre otra. |
| K11 | En el IDE, ⌘⇧, y después ⌘B | ⌘B sigue ocultando el explorador (tras recargar, la pestaña sigue en modo IDE). |
| K12 | En una pestaña en modo terminal, `vim`, ⌘P y ⌘B; `:q` | vim no inserta nada. |
| K13 | `vkterm .` en el Shell del propio IDE con otra carpeta (`vkterm ..`) | Se abre una ventana nueva con el IDE de esa carpeta. |
| K14 | En una pestaña en modo terminal (nueva, ⌘T): `claude`; captura al portapapeles (⌃⇧⌘4) y ⌘V. Después copiar un texto y ⌘V. Repetir con `codex`. Por último, en Claude Code, ⌃Z y `fg` | La imagen se adjunta (`[Image #1]`); el texto se pega como texto. ⌃Z vuelve al shell y `fg` devuelve el agente tal como estaba. |
| K15 | Con demo abierta en una pestaña, un archivo sin guardar en el editor y Claude Code en marcha: ⌘T y, en la pestaña nueva (que empieza en demo), `vkterm` (sin el punto) | El IDE de demo aparece en la pestaña nueva tal como estaba: el archivo sigue sin guardar y Claude Code sigue en marcha. En la pestaña de antes vuelve el shell, con «vkterm: …/demo pasó a otra pestaña, con todo lo que tenía abierto.» |
| K16 | En una pestaña en modo terminal, en `~`: ⌘O. Escribir parte del nombre de una carpeta; ↑ ↓; Enter. Repetir y salir con Esc, y otra vez con ⌘O | Debajo del prompt, la lista de carpetas ya abiertas (la última primero), con la carpeta de arriba y «abierta» en las que lo están. Lo escrito filtra; Enter convierte esta pestaña en el IDE de la elegida. Esc y ⌘O cierran la lista sin dejar rastro en la pantalla. |
| K17 | En una pestaña en modo terminal: ⌘O y ⇧Enter sobre una carpeta que no está abierta | Se abre una ventana nueva ya convertida en esa carpeta; la pestaña de partida sigue en el prompt. |
| K18 | En el IDE: ⌘O, elegir otra carpeta, Enter | La misma pestaña cierra la carpeta actual y abre la elegida, sin pasar por el shell. ⌘K F después vuelve al shell de antes. |
| K19 | En el IDE, escribir en el editor sin guardar; ⌘O, Enter sobre otra carpeta; Cancelar. Después ⌘K F y «Cerrar sin guardar» | Primero pregunta como ⌘K F («1 archivo sin guardar») y Cancelar deja todo como estaba. El ⌘K F de después vuelve al shell: no abre la carpeta de antes. |
| K20 | En el IDE: ⌘O y ⇧Enter sobre otra carpeta; después ⌘O y Enter sobre una marcada «abierta» | ⇧Enter abre una ventana nueva y el IDE de partida sigue igual. Con una «abierta», cierra la carpeta actual y trae aquí el IDE de la elegida; su pestaña vuelve al shell. |
| K21 | Con demo abierta en una ventana pequeña: ⌘N, pantalla completa (⌃⌘F) y `vkterm .` | El IDE de demo pasa a la ventana grande, redimensionado a ella. En la ventana pequeña vuelve el shell con el aviso. ⌘B, ⌘J, ⌘S y ⌘K ⌘S funcionan en la grande. |
| K22 | Con una carpeta abierta por una versión anterior de vkTERM (abierta antes de actualizar), `vkterm .` de esa carpeta en otra pestaña | No la trae: dice que se abrió con una versión anterior (ciérrala allí con ⌘K F) y va a su pestaña. |

## L. Cerrar la carpeta (⌘K F)

| # | Acción | Resultado esperado |
|---|---|---|
| L1 | Sin cambios: ⌘K F | Vuelve al shell sin preguntar. |
| L2 | `vkterm .`, escribir en el editor sin guardar, ⌘K F | Ventana «Cerrar carpeta»: «1 archivo sin guardar» con su ruta; botones Guardar y cerrar · Cerrar sin guardar · Cancelar. |
| L3 | Esc | La ventana se cierra; la carpeta sigue abierta. |
| L4 | ⌘K F, Enter | Guarda el archivo y vuelve al shell. |
| L5 | `vkterm .`, escribir sin guardar, ⌘K F, →, Enter | Vuelve al shell sin guardar. Al abrir otra vez, el archivo está como antes y Micro no ofrece recuperar nada. |
| L6 | ⌘3, Enter (arranca Claude Code), ⌘K F | Pregunta: «Claude Code, que está en marcha». Cancelar lo deja abierto; Cerrar lo termina. |
| L7 | Con Claude Code y Codex en «Waiting to run», ⌘K F | No pregunta por ellos. |

## M. Memoria de cada carpeta

| # | Acción | Resultado esperado |
|---|---|---|
| M1 | Abrir dos archivos (el segundo activo), desplegar `src`, ⌥⌘B, ⌘5, ⌘K F y `vkterm .` | Todo vuelve igual: las dos pestañas con la misma activa, `src` desplegada, agentes ocultos y foco en el Shell. |
| M2 | Lo mismo, pero cerrando la ventana (botón rojo, Close) en lugar de ⌘K F | Igual. |
| M3 | Mirar Claude Code y Codex | Empiezan de cero («Waiting to run»): las conversaciones no se recuerdan. |
| M4 | `vkterm . src/App.tsx` | Se abre además App.tsx, que queda activo. |

## R. Control de código fuente (⌃⇧G) y colores de git

En un repositorio con una rama que cambia la misma línea que `main` (por
ejemplo, el de `scripts/test_vkterm_git.py`) y algún cambio sin commit.

| # | Acción | Resultado esperado |
|---|---|---|
| R1 | ⌃⇧G; después ⌘⇧E | ⌃⇧G: la barra lateral pasa a «Control de código fuente» con el foco en el mensaje. ⌘⇧E: vuelve el explorador, con el foco. |
| R2 | Mirar el explorador y la barra de estado | Archivos modificados en amarillo con M, nuevos en verde con U, carpetas con cambios con •, lo ignorado apagado. Barra: «⎇ rama*» y «⚠ 1 conflicto con main». |
| R3 | ⌃⇧G, ↓ hasta un archivo de «Cambios», Espacio; Enter; q | Pasa a «Cambios preparados». Enter abre sus diferencias en una ventana flotante; q la cierra y el foco vuelve a la vista. |
| R4 | Escribir un mensaje y ⌘Enter | Commit hecho (`git log`); el mensaje se vacía. ⌘Enter **no** pone la ventana en pantalla completa. |
| R5 | Con cambios sin preparar: mensaje y Enter | Pregunta «¿Preparar todos los cambios y hacer commit?». |
| R6 | En «CONFLICTOS CON main», Enter sobre el archivo | El resultado de la fusión con sus marcas `<<<<<<< HEAD` … `>>>>>>> main`, sin haber fusionado nada (`git status` igual). |
| R7 | Enter en la línea «⚠ main …», elegir otra rama | La comparación pasa a esa rama; al volver a abrir la carpeta, sigue elegida. |
| R8 | En el Shell, `git merge main` (con conflicto) | Aparece «Conflictos de fusión» con ! en rojo, «fusión en curso» arriba y el mensaje de git propuesto. Espacio sobre el archivo sin resolver pregunta si prepararlo con marcas. ⌘⇧P «Git: Abortar la fusión» la deshace. |
| R9 | ⌘⇧P «Git: Push» en una rama sin upstream | Ventana flotante con `git push -u origin HEAD` y su salida; pide credenciales si hace falta; una tecla la cierra. |
| R10 | ⌘K ⌘T, otro tema | Los colores de git del explorador y de la vista cambian a los del tema. |

## Limpieza

Las sesiones terminan con su pestaña: no queda nada que limpiar. La memoria
de cada carpeta vive en `config/local/projects/`.

## Qué anotar si falla algo

- Si **ningún** atajo ⌘ funciona en el IDE: comprueba que es vkTERM.app y no
  un Ghostty normal, y que la barra de estado empieza por «⌨ vkTERM».
- Si ⌘B / ⌘J / ⌘1 no hacen nada pero ⌘S sí: probablemente el plugin no tiene
  permisos. Mira si el explorador muestra `Allow? (y/n)`.
- Si ⌘K ⌘S, el clic derecho o la paleta tardan unos segundos y el explorador
  avisa de que Zellij había olvidado los permisos: es lo esperado si algo
  vació `~/Library/Caches`. Si no aparecen, ejecuta `scripts/doctor.sh` y
  anota lo que dice en «Sesiones abiertas».
- Si falla solo un atajo concreto: puede interceptarlo un atajo global de otra
  app (Rectangle, utilidades del sistema). Prueba con esa app cerrada.
