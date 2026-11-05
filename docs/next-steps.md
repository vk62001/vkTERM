# Próximos pasos: GitHub y plugins

## 0g. Estudio: rehacer vkTERM en Rust, 05-11-2026 (sin decidir)

Tras los arreglos de 0.6.0 se estudió si conviene rehacer vkTERM en Rust. La
recomendación es ir por partes: primero un multiplexor propio que sustituya a
Zellij dentro de Ghostty, y decidir después si se hace una app nativa. Todo el
detalle, con cómo retomarlo, está en
[estudio-reescritura-rust.md](estudio-reescritura-rust.md).

## 0f. Edición, búsqueda en el proyecto y LSP, 03-11-2026 (sin publicar, rama `main`)

Hecho: multicursor, comentar, indentar, mover / duplicar / borrar línea,
siguiente / anterior coincidencia, ⌘⇧F (al fusionar con 0.5.0 quedó la vista
Buscar de la barra lateral),
cableado del LSP de H1 (F12, ⇧F12, ⌘I, ⌘K ⌘I, ⌘K ⌘R, ⌘K ⌘F, ⌘⇧O),
historial y blame del archivo, dividir el editor, autoguardado y «Git:
Iniciar repositorio» desde la paleta. `smoke-test.py` cubre la edición y ⌘⇧F;
`--only-editor` ejecuta solo ese escenario.

Pendiente:

1. Probar con el teclado físico y en Ghostty real: ⌘D ⌘/ ⌘] ⌘[ ⌘G ⌘L ⌘⇧K,
   ⌘⇧F, F12 ⇧F12 ⌘I ⌘K ⌘I ⌘K ⌘R ⌘K ⌘F ⌘⇧O. Los de LSP aún no tienen prueba
   automática de teclas (el LSP en sí sí se comprobó en un Micro real).
2. ⌘] y ⌘[ son las teclas físicas de esos símbolos del teclado de EE. UU.; en
   el español ISO pueden quedar en otra tecla. Decidir si se cambian.
3. Sin hacer: plegado, minimapa, migas de pan, panel de Problemas global,
   depurador, tareas, snippets y varias carpetas por ventana.


## 0e. Control de código fuente (⌃⇧G), 03-11-2026 (sin publicar, rama `temas-vscode`)

El hito H2 de la Fase 2, adelantado a petición del usuario («no veo mis
archivos modificados ni si tengo conflictos entre ramas»). Hecho: la vista
Control de código fuente en la barra lateral (⌃⇧G, ⌘⇧E vuelve al
explorador), los colores de git en el explorador, el aviso en la barra de
estado y, además de lo del plan, los conflictos que habría al fusionar `main`
(u otra rama) antes de fusionar. Detalle en [architecture.md](architecture.md)
(«Control de código fuente») y [keybindings.md](keybindings.md).

Pendiente:

1. Lista manual con el teclado físico: sección R de
   [manual-tests.md](manual-tests.md). La R4 es la importante: que ⌘Enter
   haga commit y no ponga la ventana en pantalla completa.
2. `scripts/smoke-test.py --ghostty`, que ahora también prueba ⌃⇧G, ⌘Enter y
   ⌘⇧E con teclas sintetizadas.
3. Quizá, como VS Code: un editor de diferencias lado a lado (hoy es el
   paginador de git), ver los cambios por línea en el margen del editor, y
   «Iniciar repositorio» en una carpeta sin git.

## 0d. Traer el IDE aquí y Ghostty sin el indicador, 28-10-2026 (sin publicar, rama `temas-vscode`)

Hecho: `vkterm .` en una pestaña de vkTERM.app trae el IDE de una carpeta
abierta en otra, con todo vivo; Ghostty compilado sin el botón «⌨ vkterm»
(`scripts/build-ghostty.sh`, `patches/ghostty`) y el distintivo «⌨ vkTERM»
en la barra de estado. Detalle en [architecture.md](architecture.md)
(«`vkterm`» y las decisiones).

Pendiente:

1. ~~Crear vkTERM.app con la compilación~~: hecho el 03-11-2026
   (`bin/vkterm-app install`, en /Applications, con los parches de vkTERM).
2. Lista manual con el teclado físico: A1, K8, K15, K20, K21 y K22 de
   [manual-tests.md](manual-tests.md).
3. `scripts/smoke-test.py --ghostty`, que ahora también prueba el traspaso
   entre dos ventanas: no se ejecutó (vkTERM Test.app hay que crearla de
   nuevo y pide permisos).

## 0c. «Abrir reciente» (⌘O), 27-10-2026 (sin publicar, rama `temas-vscode`)

Hecho: la lista de carpetas ya abiertas en los dos modos, Enter en la misma
pestaña y ⇧Enter en una ventana nueva, y el aviso de `vkterm .` cuando la
carpeta ya está abierta. Detalle en [architecture.md](architecture.md)
(«`vkterm`») y [keybindings.md](keybindings.md).

Pendiente:

1. Lista manual con el teclado físico: K15–K20 de
   [manual-tests.md](manual-tests.md). La K16 y la K17 son las importantes:
   que Ghostty envíe ⇧Enter como `ESC [13;2u` a la lista del modo terminal
   (en las pruebas PTY se envían esos bytes; no son pulsaciones de verdad).
2. `scripts/smoke-test.py --ghostty`: no se ejecutó, porque desde vkTERM.app
   el AppleScript a vkTERM Test pide un permiso nuevo.
3. Quizá: quitar una carpeta de la lista (VS Code tiene la ✕). Hoy sale de
   ella si se borra su memoria en `config/local/projects/`.

## 0b. Temas de VS Code, fuente y brackets (versión 0.3.0, rama `temas-vscode`)

Hecho: 24 temas de VS Code con sus colores reales (terminal, resaltado e
interfaz), selector de fuente (⌘K ⌘L), ⌘+ ⌘- ⌘0 y brackets (pareja
resaltada, guías de sangría, cierre y envolver la selección). Detalle en
[architecture.md](architecture.md) («Temas», «Fuente» y «Brackets»).

Pendiente:

1. Lista manual con el teclado físico: secciones J, N y O de
   [manual-tests.md](manual-tests.md). La N4 es la importante: que ⌘+ en el
   IDE cambie el tamaño de todas las ventanas (y no solo la pestaña, como
   haría el menú de Ghostty).
2. Colores por nivel de los brackets (Bracket Pair Colorization): Micro no
   lo permite sin modificarlo. Opciones, si se quiere: un parche propio de
   Micro (como los de µlsp) o proponerlo al proyecto Micro.
3. Reglas de sintaxis más finas para algún lenguaje, si el resaltado se
   aleja mucho del de VS Code (archivos `syntax/` propios de Micro).

## 0. Modo terminal ⇄ IDE (versión 0.2.0, 23-10-2026, rama `modo-terminal`)

vkTERM pasa a ser el terminal de todos los días (vkTERM.app) y `vkterm .`
convierte la pestaña en el IDE. La Fase 2 queda en pausa: este trabajo se
entrega de una vez, sin fases. Detalle en [architecture.md](architecture.md)
(«Modo terminal y modo IDE») y [keybindings.md](keybindings.md).

Pendiente:

1. **Fusionar `modo-terminal` en `main`** cuando la lista manual esté
   hecha. La rama está en GitHub; los cambios, en [CHANGELOG.md](../CHANGELOG.md).
2. **Abrir vkTERM.app** (en ~/Applications, o en /Applications si la mueves) desde el Dock o
   Finder. macOS pedirá acceso al disco externo; pulsa Permitir. Comprueba
   que abre una ventana: desde un proceso en segundo plano, ni Ghostty.app
   ni su copia abren la ventana inicial, porque macOS no los activa.
3. **Lista manual** con el teclado físico: secciones K (modo terminal),
   L (⌘K F) y M (memoria), además de C2 (⌃Tab en el IDE, que las pruebas
   automáticas no pueden comprobar sin traer la ventana al frente):
   [manual-tests.md](manual-tests.md).
4. `bin/vkterm-app install --link` si quieres que `~/.local/bin/vkterm`
   apunte a `bin/vkterm` (hoy apunta a `bin/vkterm-ghostty`, que ahora solo
   le pasa los argumentos).
5. Después de cada actualización de Ghostty: `bin/vkterm-app install`.

Estado al 21-10-2026 (noche). Lo hecho esta noche está en
[phase-1-report.md](phase-1-report.md), secciones 10 a 12:

- ventana de atajos (⌘K ⌘S);
- teclado normal, en el que los dos ⌥ escriben caracteres;
- temas (⌘K ⌘T).

Este documento recoge lo que falta decidir.

## 1. Verificar primero (la pantalla estaba bloqueada)

**Hecho el 22-10-2026:** `scripts/smoke-test.py --ghostty` pasa 169/169 (dos
veces). De paso se corrigieron dos cosas:

- `bin/vkterm` fallaba con proyectos de nombre largo (`session name must be
  less than 0 characters`): la ruta del socket de Zellij en macOS deja solo
  24 caracteres para el nombre de sesión. Ahora el nombre es
  `vk-<carpeta>-<huella>` y se recorta según el presupuesto real.
- ⌃Tab / ⌃⇧Tab sintetizados solo llegan a Ghostty si su ventana está en
  primer plano (AppKit los trata como atajos de pestañas de ventana). Con el
  teclado físico siempre lo está; el test trae la ventana al frente un
  instante para esa comprobación.

Queda pendiente la lista manual (secciones I, J y F).

Con la pantalla del Mac bloqueada, macOS no crea ventanas nuevas: no se
pudieron ejecutar las pruebas en Ghostty real del selector de temas ni sacar
sus capturas. Las pruebas PTY sí se ejecutaron.

1. `scripts/smoke-test.py --ghostty`, dos veces.
2. Lista manual, secciones I (atajos), J (temas) y F (⌥ Option):
   [manual-tests.md](manual-tests.md).
3. La primera vez con esta versión, el explorador pide permiso
   (`Allow? (y/n)`): hay dos permisos nuevos para los temas.

## 2. GitHub

**Ya preparado (sin publicar nada):**

- `.gitignore`: compilación del plugin (`target/`, `*.wasm`) y estado local
  (`config/local/`, y el estado antiguo de Micro en `config/micro/`).
- `.github/workflows/ci.yml`: tests y compilación del plugin, y `config.kdl`
  válido con Zellij 0.45.1, en `macos-14`. **Sin probar**: solo se puede
  ejecutar en GitHub.
- README, LICENSE (MIT) y THIRD_PARTY_NOTICES al día.
- El repositorio git local ya existe: rama `main`, sin commits y sin remoto.

**Decisiones tuyas:**

1. **Nombre y cuenta:** ¿`vkterm` en tu cuenta personal?
2. **Visibilidad:** ¿público o privado?
3. **Licencia:** decidida el 22-10-2026: GPL-3.0-or-later.
4. **Historia:** ¿un primer commit con las fases 0 y 1, o varios (Fase 0,
   Fase 1, añadidos)?

**Comandos, cuando des el visto bueno** (crear el repositorio publica el
código):

```sh
cd /Volumes/Apps/desarrollo/vkTERM
git add -A
git status            # comprobar que no entran config/local/, target/ ni *.wasm
git commit -m "vkTERM: Fase 0 y Fase 1 (interfaz tipo VS Code)"
gh repo create vkterm --private --source . --push    # o --public
```

**Después:**

- Comprobar que la CI pasa.
- Abrir issues para lo pendiente (H3 y la lista de abajo).
- Proteger `main` y trabajar con pull requests revisadas.

## 3. Plugins: propuesta

"Plugins" puede querer decir dos cosas. Propongo hacer las dos, en este orden.

### A. Paleta de comandos (⌘⇧P) y ⌘P con búsqueda — hecho el 22-10-2026

**Entregado:** `plugins/vkterm/src/palette.rs` (ventana), `src/fuzzy.rs`
(búsqueda difusa, con tests) y `src/commands.rs` (lista de comandos, con
tests). Documentado en [keybindings.md](keybindings.md). Lo que sigue es la
propuesta original.

Es la base de VS Code: todo se encuentra escribiendo. La paleta lista las
acciones de vkTERM y, más adelante, las de cada plugin. ⌘P busca archivos
mientras escribes. Ya estaba previsto en el H3.

Reutiliza las ventanas flotantes de la ventana de atajos y del selector de
tema (`src/floating.rs`).

### B. Extensiones del editor (Micro, en Lua)

Micro tiene su propio gestor de plugins. Lo que más se acerca a VS Code:

- **LSP** (autocompletar, ir a la definición, errores en línea):
  `micro-plugin-lsp` con los servidores de cada lenguaje (`typescript-language-server`,
  `rust-analyzer`, `gopls`…).
- **Formatear al guardar** (prettier, rustfmt, gofmt).

Para que todo siga dentro del proyecto, los plugins de Micro se instalarían en
`config/local/micro/plug/`, con una lista fija de versiones en el repositorio.

**Decisiones tuyas:** ¿qué lenguajes primero? ¿Instalación automática al
arrancar o con un comando (`bin/vkterm-plugins install`)?

### C. Extensiones de vkTERM (Rust → WebAssembly)

Vistas propias como el explorador, por ejemplo **Control de código fuente**
(⌘⇧G: git status, stage, commit), **Buscar en archivos** (⌘⇧F) y
**Problemas**.

- **Formato propuesto:** cada extensión en `plugins/<nombre>/`, con un
  `vkterm-plugin.kdl` que declara su nombre, su atajo ⌘ y dónde se muestra
  (barra lateral, panel inferior o flotante).
- **Arranque:** `bin/vkterm` generaría los alias y atajos de Zellij.
- **Permisos:** cada extensión pediría los suyos, como hace Zellij.

**Orden recomendado para la Fase 2:**

1. A: paleta y ⌘P.
2. B: LSP en Micro, la mayor mejora para editar.
3. C: Control de código fuente como primera extensión propia.
4. El formato para extensiones de terceros.

## 4. Temas: siguientes pasos

- **Otras ventanas de vkTERM** ya abiertas no cambian en vivo, solo al volver
  a abrirlas. Para cambiarlas habría que avisar a cada sesión con
  `zellij pipe`, lo que pide otro permiso.
- **Temas del usuario:** permitir añadir los suyos en `config/local/themes/`.
- **Más temas de VS Code:** Dark+, Light+ y alto contraste.
- **Micro:** esquemas en color verdadero por tema, para que el resaltado sea
  idéntico al de VS Code. Hoy usa la paleta de 16 colores del terminal.

## 5. Teclado: siguientes pasos

- ⌘/ (comentar) y ⌘D (seleccionar la siguiente coincidencia), como en VS Code.
  En el teclado español, ⌘/ es ⌘⇧7: hay que probar cómo lo ve Ghostty.
- ⌘F fuera del editor: hoy no hace nada. La búsqueda en la salida de un
  terminal está en Ctrl g, s, f.
- Los atajos con Alt+letra de Micro (Alt-g, Alt-n…) ya no se alcanzan con ⌥.
  Si echas alguno de menos, se le asigna un ⌘.
