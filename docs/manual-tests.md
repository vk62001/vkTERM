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

**Solo la primera vez**, el panel del explorador pregunta
`Allow? (y/n)`: haz clic en ese panel y pulsa **y**. Zellij guarda la
respuesta.

Anota ✔ / ✘ en cada fila. Si algo falla, anota también qué ves en la barra
de estado (abajo; tras Ctrl g muestra NORMAL) y en qué panel estaba el foco:
es el panel con el marco resaltado.

## A. Layout

| # | Acción | Resultado esperado |
|---|---|---|
| A1 | Mirar la ventana | Tema oscuro. Abajo, la barra de estado (`demo` a la izquierda, `⌘K F cerrar carpeta · ⌘K ⌘S atajos` a la derecha) y, encima, el indicador «⌨ vkterm» de Ghostty. |
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

## J. Temas (⌘K ⌘T)

La primera vez con esta versión, el explorador vuelve a preguntar
`Allow? (y/n)`: hay dos permisos nuevos (cambiar la configuración de Zellij y
enviar mensajes entre las instancias del plugin). Pulsa **y**.

| # | Acción | Resultado esperado |
|---|---|---|
| J1 | ⌘K ⌘T | Ventana "Tema de color" con nueve temas; ● marca el actual (Catppuccin Mocha). El resto se atenúa. |
| J2 | ↓ (Catppuccin Latte) | Cambian a la vez Ghostty (fondo claro), los marcos y la barra de estado, y Micro (barras en vídeo inverso). |
| J3 | ↓ dos veces (Dark Modern, Light Modern) | Los colores de VS Code. |
| J4 | Esc | Vuelve todo a Catppuccin Mocha. |
| J5 | ⌘K ⌘T, ↓, Enter | Se queda Catppuccin Latte. Al pasar a otro panel, el atenuado es gris claro. |
| J6 | ⌘K F y `vkterm .` otra vez | El IDE vuelve a abrirse con Catppuccin Latte (Ghostty, Zellij y Micro). |
| J6b | Después de J5, ⌘B | El explorador se oculta: tras cambiar el tema, la pestaña sigue en modo IDE. |
| J7 | ⌘K ⌘T, Inicio, Enter | Vuelve a Catppuccin Mocha. |

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
| K8 | ⌘T, y en la pestaña nueva ⌘B | ⌘T abre una pestaña en modo terminal en la carpeta de demo. ⌘B ahí lleva a la pestaña del IDE: la carpeta ya está abierta. |
| K9 | En iTerm (u otro terminal): `vkterm /ruta/a/otro/proyecto` | vkTERM pasa al frente con una ventana nueva ya convertida en ese proyecto. |
| K10 | Repetir K9 con la misma carpeta | Enfoca esa ventana; no abre otra. |
| K11 | En el IDE, ⌘⇧, y después ⌘B | ⌘B sigue ocultando el explorador (tras recargar, la pestaña sigue en modo IDE). |
| K12 | En una pestaña en modo terminal, `vim`, ⌘P y ⌘B; `:q` | vim no inserta nada. |
| K13 | `vkterm .` en el Shell del propio IDE con otra carpeta (`vkterm ..`) | Se abre una ventana nueva con el IDE de esa carpeta. |

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

## Limpieza

Las sesiones terminan con su pestaña: no queda nada que limpiar. La memoria
de cada carpeta vive en `config/local/projects/`.

## Qué anotar si falla algo

- Si **ningún** atajo ⌘ funciona en el IDE: comprueba que es vkTERM.app y no
  un Ghostty normal, y que abajo se ve el indicador «⌨ vkterm».
- Si ⌘B / ⌘J / ⌘1 no hacen nada pero ⌘S sí: probablemente el plugin no tiene
  permisos. Mira si el explorador muestra `Allow? (y/n)`.
- Si falla solo un atajo concreto: puede interceptarlo un atajo global de otra
  app (Rectangle, utilidades del sistema). Prueba con esa app cerrada.
