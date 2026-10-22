# vkTERM — Fase 2: lenguajes (LSP), Control de código fuente y extensiones

Estado: **aprobado el 22-10-2026.** Las descargas de H0 siguen pendientes de tu
visto bueno.

## Contexto

La Fase 1 dejó vkTERM con la interfaz de VS Code: explorador, editor con
pestañas, Shell abajo y agentes a la derecha. El 22-10-2026 se añadió la
paleta (⌘P y ⌘⇧P), el punto A de la propuesta de
[next-steps.md](next-steps.md#3-plugins-propuesta).

**Decisiones del usuario (2026-10-22):**

- **Alcance:** el orden de next-steps.md. B (LSP en Micro), C (Control de
  código fuente) y D (formato de extensiones de terceros), un hito cada uno.
- **Lenguajes:** TypeScript / JavaScript, Rust, Swift, Python y Go.
- **Instalación:** con un comando, `bin/vkterm-plugins install`. Nada se
  descarga sin pedirlo.
- **Añadido al aprobar el plan:** pegar imágenes para que las lean Claude Code
  y Codex (hito P).

## Hitos (cada uno se entrega para revisión)

| Hito | Contenido |
|---|---|
| P | Pegar imágenes en Claude Code y Codex con ⌘V. No necesita descargas: va primero. |
| H0 | Prueba técnica del LSP en Micro (sí/no) e instalación de las herramientas |
| H1 | LSP en el editor: sugerencias, ir a la definición, referencias, información, renombrar, errores en línea, formato |
| H2 | Control de código fuente (⌃⇧G) y colores de git en el explorador |
| H3 | Formato de extensiones de terceros, con su propia prueba técnica |

## P — Pegar imágenes en Claude Code y Codex

Los dos agentes adjuntan la imagen del portapapeles con **⌃V**: la leen ellos
mismos del portapapeles de macOS. Con ⌘V el terminal solo puede pegar texto, y
Claude Code avisa: "use control+v (not cmd+v!)".

- **⌘V** sigue pegando el texto (Ghostty) y, encadenado como ⌘C, avisa al
  plugin.
- **Si el foco está en Claude Code o Codex** y el portapapeles tiene una
  imagen y ningún texto (`osascript -e 'clipboard info'`), el plugin envía ⌃V
  a ese panel.
- En el editor y el Shell, ⌘V no cambia.
- **Verificación real:** una captura en el portapapeles, ⌘V en Claude Code y
  ver la imagen adjunta. Hace falta usar el portapapeles unos segundos.

## H0 — Prueba técnica del LSP (sí/no)

### Plugin de Micro elegido

Micro no trae LSP; hay dos plugins en Lua. Se leyó el código de los dos (no se
han probado todavía):

| | AndCake/micro-plugin-lsp | Andriamanitra/mlsp |
|---|---|---|
| Último cambio | 15-06-2024 | 14-08-2026 |
| Protocolo | Una petición a la vez por lenguaje; respuestas sin emparejar; manda el documento entero en cada tecla; `master` tiene un bucle infinito conocido | Respuestas emparejadas por id, cambios incrementales, cierra documentos |
| Errores en línea | Al llegar los de cualquier archivo borra los de la pestaña actual | Guardados por archivo |
| Servidores | Uno por lenguaje | Varios (Python: pyright + ruff) |
| Renombrar | No | Sí |
| Formatear al guardar | Sí | En una PR sin fusionar (#72), que aplica limpia |
| Licencia | MIT | MIT |

**Propuesta: mlsp** en el commit `91261a0`, con tres parches propios en el
repositorio:

1. Formatear al guardar (la PR #72).
2. Los `.tsx` se anuncian como `typescriptreact`. Micro los llama
   `typescript` y el servidor no entendería el JSX.
3. Renombrar abre en pestañas los archivos afectados que no estén abiertos,
   como VS Code (quedan sin guardar). mlsp solo cambia los abiertos y dejaría
   el proyecto roto.

### Servidores (versiones fijas)

| Lenguaje | Servidor | Formato | Instalación |
|---|---|---|---|
| TypeScript / JavaScript | typescript-language-server 6.0.2 + typescript 6.0.3 | el de TypeScript (el predeterminado de VS Code) | npm, local |
| Python | pyright 1.1.414 + ruff | ruff | npm y binario oficial con sha256, local |
| Rust | rust-analyzer | rustfmt | `rustup component add rust-analyzer` |
| Swift | sourcekit-lsp de Xcode | swift-format de Xcode | ya está |
| Go | gopls 0.24.0 | gofmt | Go 1.27 y gopls, locales (no hay Go en el Mac) |

- **TypeScript 7** (la versión nativa, hoy `latest` en npm) no trae tsserver:
  se fija la 6.0.3 y el servidor la usa siempre, aunque el proyecto tenga la 7.
- **rust-analyzer:** `~/.cargo/bin/rust-analyzer` existe pero no funciona
  (falta el componente de rustup). Es la única instalación fuera de
  `config/local/`, como el target wasm de la Fase 1.
- **Swift:** sourcekit-lsp entiende proyectos SwiftPM (`Package.swift`). En los
  `.xcodeproj` (vkBLINK, vkmonitor, vkrar) solo ayuda archivo por archivo.
  Para el proyecto completo hace falta `xcode-build-server`, que escribe un
  `buildServer.json` en tu proyecto: queda fuera de esta fase.
- **Go:** pesa unos 250 MB. Ninguno de tus proyectos es de Go, así que
  `install` sin argumentos no lo descarga: hay que pedirlo con `install go`.

Con tu aprobación, en H0 se instalan con el comando de H1 (primera versión).

### Comprobaciones

1. mlsp con los parches carga en Micro 2.0.15 desde el `MICRO_CONFIG_HOME` de
   vkTERM.
2. En un proyecto de cada lenguaje funcionan sugerencias, ir a la definición
   (también a otro archivo), referencias, información, renombrar, errores y
   formato:
   - TS: `examples/demo`;
   - Rust: el propio plugin de vkTERM, cuyo `Cargo.toml` no está en la raíz
     (el servidor arranca en la carpeta del proyecto);
   - Python, Go y un paquete SwiftPM mínimos.
3. Los errores de cada pestaña siguen ahí al cambiar de pestaña.
4. F12, ⇧F12, ⌃Space y F2 llegan a Micro a través de Ghostty y Zellij.
5. `node` está disponible en las sesiones abiertas desde Ghostty. Se arranca
   con `zsh -l`, que no lee `.zshrc`, donde suele cargarse nvm.
6. Escribir en un archivo TS de 3.000 líneas no va con retraso.
7. Los servidores terminan al cerrar Micro, también con ⌘W en la última
   pestaña, cuando el editor se relanza.

**Si mlsp falla**, el plan B es AndCake/micro-plugin-lsp con el arreglo del
bucle infinito, asumiendo sus limitaciones (errores que se borran, un servidor
por lenguaje). Se comunica antes de seguir.

## H1 — LSP en el editor (B)

### `bin/vkterm-plugins`

```
bin/vkterm-plugins install [ts python rust swift go]   sin argumentos: todos menos go
bin/vkterm-plugins status                               qué hay, versión y qué falta
bin/vkterm-plugins remove [lenguaje…]
```

- **Versiones fijas en el repositorio**, en `config/lsp/`:
  - `package.json` y `package-lock.json` (`npm ci`, con hashes de integridad);
  - `versions`: commit de mlsp y versión y sha256 de ruff, Go y gopls;
  - `patches/`: los parches de mlsp;
  - `mlsp.lua`: la configuración de vkTERM para mlsp.
- **Instala en:** `config/local/lsp/` (servidores) y
  `config/local/micro/plug/mlsp/` (plugin). Fuera de git, como el resto del
  estado.
- **Rutas absolutas:** la configuración de mlsp se genera al instalar con la
  ruta de cada servidor y la del `node` actual. El PATH del Shell no cambia.
  Si cambias de versión de node con nvm, `status` lo detecta.
- **Sin instalar,** vkTERM funciona como hoy (Micro sin LSP).
  `scripts/doctor.sh` y el comando de la paleta "LSP: Estado" dicen cómo
  instalarlo.

### En el editor

| VS Code | vkTERM | Atajo |
|---|---|---|
| Sugerencias en una lista desplegable | Micro no tiene listas desplegables: las sugerencias salen en la barra de estado; Tab y ⇧Tab las recorren | ⌃Space, ⌘I (como VS Code) |
| Ir a la definición | Abre el archivo en una pestaña (o salta a la suya) en esa línea | F12 |
| Buscar todas las referencias | Lista en una división de Micro; Enter salta | ⇧F12 |
| Mostrar información (hover) | En una división bajo el texto; ⌘W la cierra | ⌘K ⌘I |
| Cambiar el nombre del símbolo | Pide el nombre nuevo en la barra inferior | F2 |
| Errores y avisos subrayados | `>>` en el margen con el color del tipo, el texto subrayado y el mensaje abajo con el cursor en esa línea | — |
| Dar formato al documento | Igual | ⌘K ⌘F (ver abajo) |

- **F2:** hoy ⌘S envía F2 a Micro para guardar. Ese F2 lo genera Ghostty y no
  cambia. La tecla F2 física pasará a ser "cambiar el nombre", como en VS
  Code.
- **⇧⌥F**, el atajo de formato de VS Code, usa ⌥, y en vkTERM los dos ⌥
  escriben caracteres. Se propone ⌘K ⌘F, que en VS Code da formato a la
  selección.
- **Cómo llegan:**
  - F12, ⇧F12 y ⌃Space van directos a Micro (`bindings.json`).
  - ⌘K ⌘I, ⌘I, ⌘K ⌘F y F2 pasan por el controlador, como ⌘Z, y solo actúan
    con el foco en el editor.
- **Paleta:** "Ir a la definición", "Buscar todas las referencias", "Mostrar
  información", "Cambiar el nombre del símbolo", "Dar formato al documento",
  "LSP: Estado", "LSP: Reiniciar servidores" y "LSP: Ver registro".
- **Ventana de atajos (⌘K ⌘S):** nueva categoría "Lenguaje".

### Decisión pendiente: formatear al guardar

VS Code lo trae desactivado (`editor.formatOnSave`). **Recomiendo dejarlo
desactivado:** en un proyecto que use Prettier, el formato de TypeScript
cambiaría líneas que Prettier deja de otra forma. Se activa con una línea en
`config/micro/settings.json`, y ⌘K ⌘F formatea a mano.

## H2 — Control de código fuente (C)

Como la vista "Source Control" de VS Code, en la misma barra lateral que el
explorador.

### Cómo se muestra

VS Code tiene una sola barra lateral que enseña una vista cada vez
(Explorador, Buscar, Control de código fuente…). vkTERM hará lo mismo: **el
panel del explorador cambia de vista**, no se añade otro panel.

- **⌘⇧E**: vista Explorador. **⌃⇧G**: vista Control de código fuente. Son los
  atajos de VS Code en Mac. Si la barra está oculta, la muestran con esa vista
  y le dan el foco.
- **⌘B** oculta y muestra la barra con la vista que tenga.
- **⌘1** sigue enfocando la barra lateral, sea cual sea su vista.
- El título del panel cambia con la vista (`rename_plugin_pane`).

Así no cambian ni el layout, ni `override_layout`, ni el foco por identidad:
es la misma instancia del plugin con otra vista. El código se separa en
`src/explorer.rs` (lo que hoy está en `main.rs`) y `src/scm.rs`.

```
┌ CONTROL DE CÓDIGO FUENTE ──────┐
│ ⎇ main  ↑1 ↓0                  │
│ › Mensaje (⌘Enter: commit)     │
│ ▾ CAMBIOS PREPARADOS        1  │
│     App.tsx  src            M  │
│ ▾ CAMBIOS                   2  │
│     Header.tsx  src/comp    M  │
│     nuevo.ts                U  │
│ Enter diferencias · Espacio +/-│
└────────────────────────────────┘
```

### Qué hace

| VS Code | vkTERM |
|---|---|
| Rama, cambios por subir y por bajar | Cabecera: `⎇ main ↑1 ↓0` |
| Cuadro del mensaje, ⌘Enter hace commit | Línea de mensaje arriba; ⌘Enter hace commit (`git commit -m`) |
| Commit sin nada preparado: "¿preparar todo y hacer commit?" | La misma pregunta (s/n) |
| Grupos "Staged Changes" y "Changes" con contador | "Cambios preparados" y "Cambios", plegables |
| Letra y color por estado (M, A, D, U, R, conflicto) | Igual, con los colores del tema |
| Clic en un archivo: editor de diferencias | Enter o clic: `git diff` del archivo en una ventana flotante (con el paginador de git del usuario); q la cierra |
| Acciones "+" (preparar) y "−" (quitar) | Espacio prepara o quita; también "Preparar todo" en la cabecera del grupo |
| "Abrir archivo" | → abre el archivo en una pestaña del editor |
| "Descartar cambios" | Pide confirmación (s/n); en archivos nuevos los borra, como VS Code |
| Pull, Push, Sincronizar | Desde la paleta; se ejecutan en una ventana flotante para ver la salida y poder escribir credenciales |

- **Teclado:** ↑ ↓ en la lista; ↑ desde el primer archivo sube al mensaje y ↓
  baja a la lista, como en VS Code. En el mensaje se escribe con normalidad.
- **Actualización:** con `watch_filesystem()` (también ve los cambios de
  `.git`), con espera de 400 ms, y tras cada acción. Se lee con
  `git status --porcelain=v2 -z --branch`, que se interpreta en
  `src/git.rs` (lógica pura con tests).
- **Sin repositorio:** "La carpeta no tiene un repositorio git", como VS Code.
  Iniciar uno queda fuera de esta fase.
- **Errores** (un hook que rechaza el commit, un push que falla): primera línea
  en el pie de la vista; la salida completa se ve en la ventana flotante.

### Además: colores de git en el explorador

Estaba pendiente desde la Fase 1 (H3) y usa el mismo `git status`:

- Archivos modificados en amarillo con `M` a la derecha; nuevos en verde con
  `U`; conflictos en rojo con `!`.
- Las carpetas con cambios dentro, en el color del cambio.
- Los archivos ignorados por `.gitignore`, atenuados.

### Comandos y atajos nuevos

| Atajo | Comando de la paleta |
|---|---|
| ⌘⇧E | Ver: Explorador |
| ⌃⇧G | Ver: Control de código fuente |
| ⌘Enter | Git: Commit (en la vista; fuera de ella no hace nada) |
| — | Git: Preparar todo, Git: Pull, Git: Push, Git: Sincronizar |

⌘Enter sustituye a "pantalla completa" de Ghostty, que sigue en ⌃⌘F (macOS).

## H3 — Extensiones de terceros (D)

Hoy cada vista nueva (paleta, temas, Control de código fuente) se añade al
código del plugin `vkterm`. Este hito define cómo añadir vistas hechas por
otros sin tocarlo, como las extensiones de VS Code.

### Propuesta

Una extensión es un plugin de Zellij (cualquier lenguaje que compile a
WebAssembly) con un manifiesto `vkterm-extension.kdl`:

```kdl
name "hola"                          // identificador: a-z, 0-9 y guiones
title "Hola mundo"                   // título de la vista
version "0.1.0"
wasm "hola.wasm"                     // relativo al manifiesto
view "sidebar"                       // sidebar | panel | floating
shortcut "super+shift+h"             // atajo de Ghostty (opcional)
command "Hola: Saludar" message="saludar"   // entradas de la paleta
```

- **Instalar:** `bin/vkterm-plugins ext add <carpeta | url-git@commit>` copia
  la extensión a `config/local/extensions/<name>/` y comprueba el manifiesto.
  Solo acepta `.wasm` ya compilados; compilar código de terceros (`build.rs`
  ejecuta código) queda fuera. También `ext list` y `ext remove`.
- **Atajos:** `bin/vkterm` genera `config/local/extensions.ghostty`, que el
  perfil de Ghostty incluye, con un código Alt libre por extensión. El
  controlador enlaza ese código en Zellij al arrancar (`rebind_keys`, sin
  escribir en disco).
- **Paleta:** lista los comandos de las extensiones instaladas.
- **Vistas:**
  - `floating`: como la paleta, otra ventana flotante.
  - `sidebar`: ocupa el sitio del explorador. El controlador cambia un panel
    por otro sin cerrarlo (`replace_pane_with_existing_pane`), así cada vista
    conserva su estado.
  - `panel`: igual, en el sitio del Shell.
- **Contrato con vkTERM:** la extensión recibe del controlador los colores del
  tema y del atenuado en un mensaje `vkterm_theme`, para verse como el resto.
- **Permisos:** los pide Zellij al cargar la extensión, como hoy con `vkterm`.
- **Ejemplo y guía:** `examples/extensions/hola/` (Rust, una vista lateral
  mínima) y `docs/extensions.md` para quien quiera escribir una.

### Prueba técnica al empezar el hito (sí/no)

1. `replace_pane_with_existing_pane` intercambia el explorador y un plugin de
   otro `.wasm` en la barra lateral, sin cerrar ninguno. ⌘B y
   `override_layout` siguen colocándolo todo bien.
2. `rebind_keys` desde el controlador añade un atajo que funciona en LOCKED.
3. Un plugin de otro `.wasm` pide sus permisos una sola vez.

**Si (1) falla**, las vistas de extensión serían solo flotantes, como la
paleta. Se comunica antes de seguir.

Las vistas "Buscar en archivos" (⌘⇧F) y "Problemas" quedan para después de
esta fase.

## Archivos

**Nuevos:**

- `bin/vkterm-plugins` (H1; `ext` en H3).
- `config/lsp/`: versiones, parches y configuración de mlsp (H1).
- `plugins/vkterm/src/explorer.rs` (sale de `main.rs`), `src/scm.rs` y
  `src/git.rs` (H2).
- `examples/extensions/hola/` y `docs/extensions.md` (H3).
- `docs/phase-2-report.md`, al entregar.

**Modificados:**

- `config/micro/bindings.json`: F12, ⇧F12, ⌃Space y los códigos internos del
  controlador (H1).
- `config/ghostty/vkterm.ghostty`: ⌘I, ⌘K ⌘I, ⌘K ⌘F, F2 (H1); ⌘⇧E, ⌃⇧G,
  ⌘Enter (H2); inclusión de `extensions.ghostty` (H3).
- `config/zellij/config.kdl`: los códigos Alt nuevos.
- `plugins/vkterm/src/`: `main.rs`, `commands.rs` y `shortcuts.rs`.
- `bin/vkterm` y `bin/vkterm-editor`.
- `scripts/doctor.sh` y `scripts/smoke-test.py`.
- `README.md`, `THIRD_PARTY_NOTICES.md` (mlsp y los servidores), `docs/`.

## Verificación

- **`cargo test`:** interpretar `git status`, decoraciones del explorador,
  comandos nuevos y su presencia en la ventana de atajos.
- **`smoke-test.py` (PTY):**
  - `vkterm-plugins status` en una carpeta de estado vacía;
  - mlsp: definición en otro archivo, errores, renombrar y formato en el
    proyecto demo, si el LSP está instalado (si no, se marca como saltado y
    no como correcto);
  - vista de Control de código fuente en un repositorio temporal: preparar,
    quitar, commit, descartar con confirmación y diferencias;
  - colores de git en el explorador;
  - los procesos de los servidores terminan con Micro.
- **`--ghostty`:** cada atajo nuevo, con teclas sintetizadas; capturas de la
  ventana real de cada vista.
- **Lista manual** (`docs/manual-tests.md`): secciones nuevas K (lenguaje), L
  (Control de código fuente) y M (extensiones). ⌃Space se prueba a mano,
  porque macOS lo usa para cambiar de fuente de entrada.

## Reglas que se mantienen

- Sin sudo ni forks. Nada se instala fuera de `config/local/` salvo lo que
  cada herramienta guarda por su cuenta (caché de npm y de cargo).
- Nada de configuración global: `~/.config/micro`, `~/.config/zellij`,
  `~/.config/ghostty` y los rc del shell no se tocan.
- Cada hito se entrega para revisión. No se pasa al siguiente sin el visto
  bueno.
- **Sin commits salvo que lo pidas.** Propongo una rama `fase-2` y un commit
  por hito aprobado; `main` ya está publicada en GitHub.
- Teclado: los dos ⌥ siguen escribiendo caracteres. Todo lo nuevo se alcanza
  con ⌘, con teclas de función o desde la paleta.
- Pruebas: `smoke-test.py` y `--ghostty` antes de cada entrega, con capturas
  de la ventana real. Las teclas se sintetizan; lo que solo cubre el teclado
  físico va a `docs/manual-tests.md`.
