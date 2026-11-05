# Cambios

Los cambios de cada versión de vkTERM. El formato sigue
[Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y las versiones,
[SemVer](https://semver.org/lang/es/). La versión actual está en `VERSION`
(`vkterm --version`).

## Sin publicar

## 0.6.0 — 2026-11-05

### Añadido

- **Copiar todo el historial del panel (⌘K ⌘C):** vuelca al portapapeles el
  historial completo de Claude Code, Codex o el Shell, para copiar prompts y
  respuestas largos sin seleccionar con el ratón (al girar la rueda la
  selección se perdía). También está en la paleta («Ver: Copiar todo el
  historial del panel»). `Alt +` deja de ser «agrandar panel»; queda `Alt =`.
- **Edición como VS Code:** multicursor (⌘D), comentar (⌘/), indentar y quitar
  sangría (⌘] ⌘[), siguiente / anterior coincidencia (⌘G ⌘⇧G), seleccionar y
  borrar la línea (⌘L ⌘⇧K), principio / fin del archivo (⌘↑ ⌘↓), mover y
  duplicar la línea (⌥↑ ⌥↓ ⌥⇧↑ ⌥⇧↓). ⌘D deja de ser «nada» en el IDE.
- **Lenguaje (LSP, H1):** F12 definición, ⇧F12 referencias, ⌘I sugerencias,
  ⌘K ⌘I información, ⌘K ⌘R cambiar el nombre, ⌘K ⌘F formato, ⌘⇧O símbolos, y
  los comandos «Lenguaje: …» de la paleta. Necesita `bin/vkterm-plugins install`.
- **Comandos nuevos de la paleta (⌘⇧P):** «Git: Historial del archivo»,
  «Git: Blame del archivo» (ventana flotante con el paginador de git),
  «Git: Iniciar repositorio», «Editor: Dividir a la derecha / abajo», «Editor:
  Ir a la línea…» y «Archivo: Activar / Desactivar el autoguardado».

### Corregido

- **ssh desde el Shell del IDE: ⌃L, `clear`, nano y htop no funcionaban en el
  servidor.** El arreglo de abajo solo valía en el modo terminal: los paneles
  del IDE heredaban `TERM=xterm-ghostty` y sus shells no cargan la integración
  de Ghostty que envuelve `ssh`. Ahora los paneles ven `TERM=xterm-256color`
  (lo que emula Zellij; bloque `env` de config/zellij/config.kdl).
- **Los atajos ⌘ del IDE (también ⌘C en el editor) dejaban de funcionar** tras
  cualquier recarga de la configuración de Ghostty que no hacía vkTERM:
  «Reload Configuration» del menú o macOS al pasar de claro a oscuro. Ghostty
  apaga entonces el modo IDE de todas las pestañas y nada lo volvía a
  encender. Ahora cada pestaña del IDE tiene un vigilante
  (`vkterm-surface keep`) que lo enciende de nuevo en 2 segundos.
- **Edición › Copiar no mostraba ⌘C** (Ghostty 1.3.1 lo trae `performable` y
  esos atajos no van al menú). Ahora lo muestra; sin selección, ⌘C no hace
  nada, como en Terminal. Pegar sigue sin mostrar ⌘V: tiene que ser
  `performable` para adjuntar imágenes a Claude Code y Codex.
- **El clic derecho en el explorador, y en el editor con el foco en otro
  panel, no hacía nada.** Zellij 0.45.1 solo pasa el clic derecho al panel que
  ya tiene el foco y nunca a un plugin. `scripts/build-zellij.sh` compila
  Zellij con un parche pequeño (patches/zellij) y `bin/vkterm-workspace` lo
  usa si existe; sin compilarlo, sigue como antes (⌘. y ⇧F10).
- **Seleccionar con el ratón en un panel sin el foco no copiaba nada** (y ⌘C
  después tampoco): el primer arrastre solo enfocaba el panel. Ahora el mismo
  clic enfoca y llega al panel, como en VS Code: en el Shell selecciona y
  copia al soltar, en el editor pone el cursor y en el explorador elige el
  archivo (`mouse_click_through` en config/zellij/config.kdl).
- **nano, htop y compañía fallaban por ssh** con «Error opening terminal:
  xterm-ghostty»: los servidores no tienen ese terminfo. Ahora `ssh` desde una
  pestaña lo instala en el servidor la primera vez (`~/.terminfo`) y, si no
  puede, entra con `TERM=xterm-256color`.
- **⌘K ⌘S, el clic derecho, la paleta y las demás ventanas dejaban de
  abrirse** si algo vaciaba `~/Library/Caches` (un limpiador, o macOS cuando
  falta espacio) con una carpeta abierta: Zellij olvidaba los permisos del
  plugin y cada ventana nueva se quedaba escondida pidiendo `Allow? (y/n)`.
  Ahora `bin/vkterm-permissions` los vuelve a escribir al abrir una carpeta y,
  con la carpeta abierta, cada 10 segundos; si aun así una ventana se queda
  pidiéndolos, el explorador le contesta «y» y se abre sola, con un aviso.
  `scripts/doctor.sh` revisa las sesiones abiertas (permisos, ventanas
  atascadas, plugin recompilado después de abrirlas).
- **Las pruebas automáticas dejaban sin atajos ⌘ las pestañas del IDE** del
  vkTERM.app desde el que se ejecutaban: al cambiar de tema, `vkterm-theme`
  subía por los procesos padre del pseudoterminal de la prueba hasta ese
  Ghostty y lo recargaba (Ghostty vacía las tablas de atajos al recargar).
  Ahora solo recarga el Ghostty que dibuja el terminal de la sesión, y la
  prueba usa un identificador de app que no existe para AppleScript.
- **El clic derecho en el explorador no abría su menú** (desde 0.4.0): Zellij
  0.45.1 no les pasa el clic derecho a los plugins (su rama principal ya lo
  hace). Ahora el menú se abre con **⌘.**, desde cualquier panel (muestra el
  explorador si estaba oculto), o con **⇧F10** en el explorador, como en VS
  Code, para lo seleccionado y debajo de ello. También desde la paleta («Ver:
  Menú del clic derecho del explorador»). El clic derecho funcionará cuando
  Zellij publique su arreglo; el del editor sí funciona.
- **El explorador no mostraba los archivos creados desde fuera** (por Claude
  Code, Codex o el Shell): dependía solo de los avisos de Zellij, que no
  siempre llegan. Ahora relee además cada segundo las carpetas abiertas y solo
  redibuja si algo cambió.

### No hecho (a propósito)

Plegado de código, minimapa, migas de pan, panel de Problemas global,
depurador, tareas y snippets: Micro no los tiene y no se pueden añadir sin
tocar Micro. Los símbolos del archivo (⌘⇧O) sustituyen al esquema.

## 0.5.0 — 2026-11-03

Buscar y reemplazar, como en VS Code.

### Añadido

- **Buscar en los archivos (⌘⇧F)**: una vista más de la barra lateral, junto
  al explorador y Control de código fuente (⌘⇧E vuelve al explorador).
  - Escribir busca en toda la carpeta (con ripgrep, que respeta `.gitignore`
    y mira también los archivos ocultos como `.env`). Los resultados salen
    agrupados por archivo, plegables, con la coincidencia resaltada.
  - Ctrl t, Ctrl w y Ctrl r: distinguir mayúsculas, palabras completas y
    expresión regular; las activas se marcan arriba a la derecha.
  - Enter (o un clic) en una coincidencia abre el archivo en esa línea y
    columna; en un archivo, pliega o despliega sus coincidencias.
  - **Reemplazar en los archivos:** Tab muestra el campo de reemplazo y Enter
    en él reescribe todos los archivos con coincidencias, tras confirmar
    (con regex valen `$1`…). Pide ripgrep (`brew install ripgrep`); sin él,
    solo busca, con grep. Los archivos abiertos en el editor con cambios sin
    guardar hay que recargarlos.
  - `bin/vkterm-search` (también en «Ver: Buscar en los archivos» de ⌘⇧P) y
    `vkterm-open` entiende `VKTERM_LINE` y `VKTERM_COLUMN`.
- **Reemplazar en el archivo (⌥⌘F)**, el Ctrl+H de VS Code: pide qué buscar
  (la selección, si la hay) y con qué, y si reemplazar todas a la vez o una a
  una (y: esta, n: saltar, Esc: parar). Es texto literal y no distingue
  mayúsculas. También en ⌘⇧P: «Editor: Reemplazar en el archivo». (⌘F ya
  buscaba en el archivo.)
- `scripts/doctor.sh` comprueba ripgrep.

## 0.4.2 — 2026-11-03

### Añadido

- **«Mostrar en Finder»** en el menú del clic derecho, como «Reveal in Finder»
  de VS Code (`open -R`): en el editor, el archivo de la pestaña (si aún no
  está guardado, lo avisa); en el explorador, la entrada seleccionada. Sin
  atajo de teclado.

## 0.4.1 — 2026-11-03

### Corregido

- **`vkterm` no abría el IDE en la 0.4.0:** `bin/vkterm-workspace` copiaba mal
  la configuración de Micro (faltaba un espacio en el `cp`) y salía con error;
  como el shell borra la pantalla al terminar, el mensaje no se llegaba a leer.

## 0.4.0 — 2026-11-03

### Añadido

- **Comentarios atenuados en `.env` y JSON**, como en VS Code. Micro no
  resalta los `.env` y su JSON no entiende comentarios, así que todo salía
  del mismo color. Ahora `config/micro/syntax` aporta:
  - `env.yaml`: `.env`, `.env.local`, `.env.*` y `*.env`. Las líneas
    comentadas (`#OTRA=1`) y los comentarios tras un valor van en el color de
    comentario del tema; la clave, el `=` y el valor (con sus `$VARIABLES`)
    tienen el suyo;
  - `json.yaml`: JSON con comentarios (`//` y `/* */`), como
    `tsconfig.json` o `.vscode/settings.json`, también para `.jsonc`,
    `.json5` y `.code-workspace`.

  `bin/vkterm-workspace` copia la carpeta en cada arranque, así que se aplica
  a todos los temas sin regenerar nada.

- **Renombrar (⌘R)** el archivo o carpeta seleccionado del explorador, o
  «Archivo: Renombrar…» en ⌘⇧P. VS Code usa F2, que no todos los teclados
  tienen. La ventana es la de «Nuevo archivo» con el nombre actual escrito;
  con `/` mueve a otra carpeta (la crea si no existe), no pisa nada que ya
  exista y admite cambiar solo mayúsculas. Un archivo renombrado se abre con
  su nombre nuevo; la pestaña del editor con el nombre viejo hay que cerrarla.
- **Clic derecho en el explorador**, como en VS Code: selecciona la entrada y
  abre junto al puntero un menú con Nuevo archivo… (⌘N), Renombrar… (⌘R),
  mostrar u ocultar .git y la paleta de comandos. Es el mismo menú flotante
  del editor, con otras entradas.
- **Control de código fuente (⌃⇧G)**, como la vista "Source Control" de VS
  Code, en la misma barra lateral que el explorador (⌘⇧E vuelve a él; ⌘B
  oculta la barra con la vista que tenga):
  - la rama, los commits por bajar y por subir y la fusión (o rebase) en
    curso;
  - el mensaje del commit: Enter o ⌘Enter hace commit; sin nada preparado,
    pregunta si preparar todo. Durante una fusión propone el mensaje de git;
  - los grupos «Conflictos de fusión», «Cambios preparados» y «Cambios», con
    la letra y el color de VS Code. Espacio prepara o quita (en la cabecera,
    todo el grupo), Enter muestra las diferencias en una ventana flotante con
    el paginador de git, → abre el archivo y ⌫ descarta los cambios (o borra
    el archivo nuevo), preguntando antes. Marcar como resuelto un archivo que
    aún tiene marcas de conflicto también pregunta;
  - **conflictos entre ramas antes de fusionar:** la vista compara la rama con
    `main` (o la principal; «Git: Comparar con otra rama…» elige otra, que se
    recuerda por carpeta) con `git merge-tree`, sin tocar el árbol de
    trabajo: cuántos commits faltan en cada lado, si habría conflictos y en
    qué archivos. Enter en uno muestra el resultado con sus marcas
    (`<<<<<<<`);
  - en ⌘⇧P: Git: Commit, Preparar todo, Quitar todo, Pull, Push,
    Sincronizar, Fetch, Comparar con otra rama, Abortar la fusión y
    Actualizar. Pull, Push y Sincronizar se ven en una ventana flotante, donde
    se pueden escribir credenciales; una rama sin upstream se publica en el
    único remoto.
- **Colores de git en el explorador**, como VS Code: el nombre en el color de
  su estado (los `gitDecoration.*` de cada tema) con la letra a la derecha (M,
  U, A, D, R, !), • en las carpetas con cambios dentro y lo ignorado apagado.
- La barra de estado muestra "*" junto a la rama si hay cambios, los commits
  por bajar y por subir, y avisa de los conflictos: «⚠ 1 conflicto con main»
  o «⚠ fusión: 2 conflictos».
- `bin/vkterm-git`, con sus tests (`scripts/test_vkterm_git.py`, también en
  CI).

- **«Abrir reciente» (⌘O)**, como «File: Open Recent» de VS Code: la lista
  de las carpetas ya abiertas en vkTERM, la última usada primero, con su
  carpeta de arriba y «abierta» o «no disponible». Escribir filtra; Enter la
  abre en esta pestaña (si ya está abierta en otra, trae aquí su IDE) y
  ⇧Enter en una ventana nueva (si ya está abierta, va a su pestaña).
  - En el modo terminal, ⌘O en el prompt (también en `~`) o `vkterm -r`: la
    lista aparece debajo del prompt (`bin/vkterm-recent`).
  - En el IDE, ⌘O o «Archivo: Abrir reciente…» en ⌘⇧P. Enter cierra la
    carpeta actual, preguntando como ⌘K F si se perdería algo, y abre la
    elegida en la misma pestaña, sin volver al shell.
- **Traer el IDE a otra pestaña o ventana.** En vkTERM.app, `vkterm .` (o
  `vkterm`, que es lo mismo, o ⌘B ⌘J ⌘P ⌘⇧P en el prompt) de una carpeta que
  ya está abierta en otra pestaña trae su IDE con todo vivo: Claude Code,
  Codex, el Shell y los archivos sin guardar. La otra pestaña vuelve a su
  shell y avisa. Así se pasa el IDE a una ventana grande. La otra pestaña
  suelta la sesión de Zellij (⌥` → `Detach`, `bin/vkterm-surface detach`) y
  la nueva se adjunta (`bin/vkterm-workspace --attach`). Las carpetas
  abiertas antes de esta versión no se pueden traer: se va a su pestaña.
- **Sin el botón «⌨ vkterm» de Ghostty sobre el IDE.** Ghostty 1.3 lo dibuja
  mientras una pestaña es el IDE y no se puede ocultar ni mover.
  `scripts/build-ghostty.sh` compila Ghostty con `patches/ghostty`, que lo
  quita (las secuencias a medias, como ⌘K …, se siguen viendo), y
  `bin/vkterm-app install` usa esa compilación si existe para la versión de
  Ghostty.app. Sortea que Zig 0.15 no enlaza con los SDK de macOS 26.4+
  (ziglang/zig#31665). Necesita Xcode 26 con su Metal Toolchain.
- La barra de estado del IDE empieza por el distintivo «⌨ vkTERM», como el de
  la izquierda de VS Code.
- `vkterm --window`: abre la carpeta en una ventana nueva de vkTERM.app.
- `vkterm-state recent`: las carpetas con memoria, la última usada primero.

### Cambiado

- ⌘Enter ya no es la pantalla completa de Ghostty (sigue en ⌃⌘F, como en las
  demás apps de macOS): en el IDE es el commit y en el modo terminal no hace
  nada. Con ⌘Enter en su menú, macOS lo resolvería antes que el IDE.
- La ventana de atajos (⌘K ⌘S) pone la nueva categoría en una tercera columna
  si cabe; si no, en dos, y si tampoco, en una.
- Desde otro terminal o desde el Shell de otro IDE, `vkterm .` de una carpeta
  que ya está abierta lo dice («ya está abierta en otra pestaña: vamos a
  ella») en lugar de cambiar de pestaña sin avisar. En el Shell del IDE de esa
  misma carpeta, dice que ya está abierta aquí.
- Si vkTERM.app no encuentra la pestaña desde la que se escribió `vkterm .`,
  lo dice antes de abrir una ventana nueva (antes la abría sin avisar).
- `--new` solo vale con `--here`. En vkTERM.app, la segunda sesión de una
  carpeta pisaba el registro de la pestaña de la primera, que dejaba de
  reactivar sus atajos ⌘ tras cambiar de tema o de fuente.

## 0.3.0 — 2026-10-23

Temas de VS Code con sus colores reales, selector de fuentes y brackets como
en VS Code.

### Añadido

- **24 temas de VS Code** en ⌘K ⌘T (antes 9):
  - los de serie: Dark+, Light+, Dark y Light High Contrast, Monokai,
    Solarized Dark y Light, Abyss, Quiet Light, Kimbie Dark y Tomorrow Night
    Blue, más Dark Modern y Light Modern;
  - los del Marketplace: Catppuccin Mocha y Latte, Dracula, Nord, Tokyo
    Night, Gruvbox oscuro y claro, SynthWave '84, Shades of Purple, Cobalt2 y
    Monokai Pro. Monokai Pro es de pago, así que se usa una aproximación
    hecha con su paleta.
- **Los colores reales de cada tema.** Se aplican en tres sitios:
  - el terminal;
  - el resaltado del editor, en color de 24 bits;
  - la interfaz: barra lateral, selección, paleta, barra de estado (azul en
    Dark+) y borde del panel con el foco.

  `scripts/gen-themes.py` lo genera todo a partir de los temas de VS Code
  (`config/themes/`), con los valores por defecto de VS Code para lo que el
  tema no define. La CI comprueba que lo generado está al día.
- **El selector de tema funciona como el de VS Code:** los temas van
  agrupados en claros, oscuros y alto contraste, escribir filtra y la lista
  se desplaza.
- **Selector de fuente (⌘K ⌘L)** con las fuentes monoespaciadas instaladas.
  Prueba cada fuente en vivo y ← → cambian el tamaño (`bin/vkterm-font`).
- **⌘+, ⌘- y ⌘0** cambian el tamaño de letra en todas las ventanas, y el
  tamaño se recuerda. Sustituyen a los de Ghostty: macOS resolvía ⌘- y ⌘0 por
  el menú antes que por la tabla del IDE. En el modo terminal funcionan desde
  el prompt de zsh.
- **⌘V pega imágenes también en el modo terminal**, en Claude Code y Codex,
  como en el IDE. `claude` y `codex` corren a través de `bin/vkterm-agent`,
  que les convierte en ⌃V el ⌘V que Ghostty no puede pegar. `command claude`
  se lo salta.
- **Brackets:**
  - la pareja del bracket se resalta con el color del tema;
  - hay guías de sangría en cada nivel;
  - al escribir un bracket se añade el de cierre;
  - con texto seleccionado, `(` `[` `{` y las comillas lo envuelven.
- **⌘N «Nuevo archivo…»**, como el del explorador de VS Code: pide el nombre
  y crea el archivo vacío en la carpeta seleccionada del explorador (o en la
  del archivo seleccionado). Con `/` crea también las carpetas
  (`ui/Boton.tsx`) y avisa si ya existe. El archivo queda seleccionado en el
  explorador y se abre en una pestaña. También en ⌘⇧P, donde la pestaña sin
  nombre de antes pasa a «Archivo: Nuevo archivo de texto sin título».
- **Menú contextual en el editor (clic derecho)**, como el de VS Code: Cortar,
  Copiar, Pegar, Seleccionar todo y Paleta de comandos…, junto al puntero.
  Antes el clic derecho no hacía nada en el IDE. Fuera de la selección, el
  cursor va antes al sitio del clic; dentro, la selección se conserva.
- **`scripts/build-dmg.sh`** empaqueta la vkTERM.app instalada en
  `dist/vkTERM-<versión>.dmg`, para instalarla arrastrándola a Aplicaciones en
  otro Mac preparado para vkTERM (con el repositorio en la misma ruta).

### Cambiado

- **Cambiar de tema ya no escribe teclas en el editor.** El plugin Lua de
  Micro sigue el tema guardado. Los cambios caros (recargar Ghostty) se
  aplican de uno en uno y siempre queda el último.
- **Los temas de alto contraste no atenúan** los paneles sin foco.
- **`bin/vkterm-theme set ID`** solo recibe el id. Ghostty, Zellij y Micro
  usan `vkterm-<id>`.
- **⌘N en el IDE crea un archivo y ⌘⇧N abre una ventana**, como en VS Code.
  En el modo terminal, ⌘N sigue abriendo una ventana. ⌘N se enlaza por tecla
  física (`super+key_n`), como ⌘T: si no, macOS lo resolvería por el menú
  (New Window) antes que por la tabla del IDE.

### Corregido

- **La ventana de atajos (⌘K ⌘S) tiene los colores del tema elegido.** Usaba
  el fondo de la paleta (casi negro en Kimbie Dark), un marco gris y las
  teclas en el gris por defecto de VS Code, igual que las descripciones. Ahora
  es como el editor «Keyboard Shortcuts» de VS Code: el fondo y el texto del
  editor del tema y cada atajo en una etiqueta (`keybindingLabel.*`).
- **Los atajos ⌘ del IDE funcionan con el panel subido con la rueda.** Al
  releer la salida de Claude Code, Zellij 0.45 pasaba al modo Scroll: ⌘K ⌘S,
  ⌘K ⌘T, ⌘1–⌘5 y los demás no hacían nada, y lo que se escribía lo tomaban
  sus atajos. Ahora (`scroll_mode_sync false`) se queda en locked, como el
  terminal de VS Code: al escribir, el panel vuelve abajo.
- **Salir de vkTERM justo después de cerrar una carpeta ya no la vuelve a
  abrir.** `bin/vkterm-surface` revisa la pestaña unos segundos después y,
  con la app cerrada, su AppleScript la lanzaba otra vez.
- **`scripts/smoke-test.py`** abre sus shells sin la pregunta de oh-my-zsh
  por actualizarse, que se comía la primera tecla de lo que escribe la
  prueba.

### Problemas conocidos

- **Los brackets no cambian de color según su nivel de anidado.** Micro no lo
  permite sin modificarlo.
- **El resaltado se parece mucho al de VS Code, pero no es idéntico.** Los
  colores son los del tema, pero qué palabra recibe cada uno depende de las
  reglas de sintaxis de Micro.
- **⌘+ en el modo terminal** solo funciona en el prompt: dentro de vim o
  less no hace nada.

## 0.2.0 — 2026-10-23

vkTERM pasa a ser el terminal de todos los días: una pestaña normal se
convierte en el IDE con `vkterm .` y vuelve al terminal con ⌘K F.

### Añadido

- **vkTERM.app** (`bin/vkterm-app install [--link]`): una copia local de tu
  Ghostty.app con nombre, icono e identificador propios. Va firmada en local y
  carga el perfil de vkTERM. Tu Ghostty y `~/.config/ghostty` no cambian.
  Se crea en `~/Applications` y se puede mover a `/Applications`.
- **Modo terminal y modo IDE, por pestaña.** El modo terminal es tu zsh
  normal, con las pestañas (⌘T) y divisiones (⌘D) de Ghostty. Para pasar al
  IDE:
  - `vkterm .` o `vkterm RUTA`;
  - ⌘B, ⌘J, ⌘P o ⌘⇧P en el prompt, que convierten la pestaña con la carpeta
    actual y después hacen su acción.

  El shell de partida queda debajo, intacto.
- **⌘K F «Cerrar carpeta»**, también en la paleta. Cierra Micro, Claude Code y
  Codex y vuelve al prompt de antes, como al salir de vim. Solo pregunta si se
  perdería algo, es decir, archivos sin guardar o un agente en marcha. Las
  opciones son Guardar y cerrar · Cerrar sin guardar · Cancelar.
- **Memoria por carpeta, como VS Code.** Al cerrar la carpeta o la ventana,
  vkTERM recuerda:
  - las pestañas del editor y cuál está activa;
  - las zonas visibles y el foco;
  - las carpetas desplegadas y la selección del explorador.

  Las conversaciones de los agentes no se recuerdan.
- **Barra de estado** al pie del IDE, con el proyecto, la rama de git, el modo
  de Zellij y los atajos para cerrar la carpeta (⌘K F) y ver los atajos
  (⌘K ⌘S). Sustituye a la `compact-bar` de Zellij.
- **`vkterm RUTA` desde otro terminal** (o desde el Shell del IDE) abre una
  ventana de vkTERM.app ya convertida. Si la carpeta ya está abierta, va a su
  pestaña.
- **Integración con zsh** (`config/zsh/`): se carga después de tu `.zshrc` y
  convive con la integración de shell de Ghostty.
- **LSP en Micro** (`bin/vkterm-plugins install|status|remove`), con
  [µlsp](https://github.com/Andriamanitra/mlsp) y versiones fijas en
  `config/local/`:
  - para TypeScript/JavaScript, Python, Rust, Swift y Go;
  - errores en línea, información, ir a la definición, referencias,
    completado con Tab y formato.
- **⌘V pega imágenes** en Claude Code y Codex: si el portapapeles solo tiene
  una imagen, la adjunta.
- **Comandos de la paleta:** «Archivo: Cerrar carpeta» y «Preferencias:
  Recargar la configuración de Ghostty» (⌘⇧,).
- **`VERSION`, `vkterm --version` y este archivo.** `scripts/doctor.sh`
  muestra la versión y el estado de vkTERM.app.

### Cambiado

- **`bin/vkterm` es el nuevo comando `vkterm`.** Decide dónde abrir el IDE:
  - en la propia pestaña;
  - en una ventana nueva;
  - o va a la pestaña donde la carpeta ya está abierta.

  El antiguo `bin/vkterm` pasa a `bin/vkterm-workspace`. `bin/vkterm-ghostty`
  queda como alias. `vkterm --here` abre el IDE en el terminal actual, para
  otros emuladores.
- **Las sesiones ya no sobreviven a la ventana.** Cerrar la pestaña o la
  ventana termina la sesión de Zellij (`on_force_close "quit"`) y lo
  importante queda en la memoria de la carpeta. Se quita Detach.
- **Cada sesión usa la configuración de su versión.** Es una copia de
  `config/zellij` por huella: al actualizar vkTERM, una sesión abierta no
  mezcla el plugin antiguo con el nuevo.
- **Perfil de Ghostty:**
  - los atajos del IDE pasan a la tabla `vkterm/…`, que solo está activa en
    las pestañas convertidas;
  - sin `title` fijo ni `maximize`;
  - sin actualizaciones automáticas;
  - con `config/local/user.ghostty` opcional para tus ajustes.
- **Icono nuevo**: `/>Vk` en verde neón sobre grafito (`docs/assets/`).
- **⌘T y ⌘⇧, se enlazan por tecla física**, porque macOS los mandaba al menú
  de Ghostty antes que a la tabla del IDE.
- **Micro:**
  - `bin/vkterm-micro` reúne la forma de mandarle comandos;
  - un plugin Lua propio guarda las pestañas del editor para la memoria.

### Corregido

- `vkterm .` desde un panel de vkTERM ya no se niega a arrancar por heredar
  las variables de Zellij.
- Un `mode` desconocido del plugin cierra esa instancia, en vez de abrir un
  segundo explorador.

### Problemas conocidos

- Tras cada actualización de Ghostty hay que repetir `bin/vkterm-app install`.
  macOS puede volver a pedir los permisos de vkTERM.
- Algunos menús siguen diciendo «Ghostty» (por ejemplo, «Quit Ghostty»).
- Recargar la configuración desde el menú de Ghostty saca la pestaña del modo
  IDE. ⌘⇧, y el cambio de tema la reactivan.
- Los atajos que convierten solo funcionan en el prompt de zsh.
- Las comprobaciones automáticas usan teclas sintetizadas. La lista con
  teclado físico está en [docs/manual-tests.md](docs/manual-tests.md)
  (secciones K, L, M y C2).

## 0.1.0 — 2026-10-22

Primera versión publicada: Fase 0 y Fase 1, un IDE de terminal que se maneja
como VS Code.

### Añadido

- **Espacio de trabajo** con Ghostty, Zellij y Micro:
  - explorador, editor con pestañas, Shell, Claude Code y Codex;
  - comando global `vkterm .`.
- **Plugin propio `vkterm`** (Rust → WebAssembly):
  - explorador de archivos;
  - mostrar u ocultar zonas (⌘B, ⌘J, ⌥⌘B);
  - foco por número (⌘1–⌘5) y foco visible atenuando los demás paneles.
- **Ventana de atajos** (⌘K ⌘S) y **selector de temas** (⌘K ⌘T). Hay nueve
  temas, que se aplican a Ghostty, Zellij y Micro.
- **⌘P y ⌘⇧P:**
  - ⌘P abre archivos con búsqueda difusa;
  - ⌘⇧P abre la paleta de comandos, como VS Code.
- **Teclado de macOS:** los dos ⌥ escriben caracteres y las acciones van por ⌘.

### Corregido

- Los nombres de sesión respetan el límite de la ruta del socket de Zellij en
  macOS: antes fallaba con proyectos de nombre largo.
