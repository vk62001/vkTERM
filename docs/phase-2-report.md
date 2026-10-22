# Informe de la Fase 2

Plan aprobado: [phase-2-plan.md](phase-2-plan.md). Equipo: Mac mini Apple M4,
teclado Spanish - ISO.

| Hito | Estado |
|---|---|
| P — Pegar imágenes en Claude Code y Codex | Entregado el 22-10-2026 |
| H0 — Prueba técnica del LSP | **Sí: mlsp sirve.** Entregado el 22-10-2026 |
| H1 — LSP en el editor | Siguiente |
| H2 — Control de código fuente | Pendiente |
| H3 — Extensiones de terceros | Pendiente |

## P — Pegar imágenes en Claude Code y Codex

Los dos agentes adjuntan la imagen del portapapeles con ⌃V; con ⌘V un terminal
solo puede pegar texto. Ahora ⌘V pega el texto como siempre y, si el foco está
en Claude Code o Codex y el portapapeles solo tiene una imagen, el plugin les
manda ⌃V (`src/clipboard.rs`).

- **Verificado** en Ghostty real con teclas sintetizadas: con una imagen, ⌘V
  en Claude Code muestra `[Image #1]`; con texto, pega el texto y no adjunta
  otra imagen.
- **Sin probar en real:** Codex (su propio aviso dice que usa ⌃V). Está en la
  lista manual (D8).

## H0 — Prueba técnica del LSP

### Qué se instaló

`bin/vkterm-plugins install` (primera versión), con las versiones fijas de
`config/lsp/`. En este Mac tardó 8 s y ocupa 81 MB en `config/local/lsp/`.

| Lenguaje | Servidor | Formato |
|---|---|---|
| TypeScript / JavaScript | typescript-language-server 6.0.2 + TypeScript 6.0.3 | el de TypeScript |
| Python | pyright 1.1.414 + ruff 0.16.10 | ruff |
| Rust | rust-analyzer 1.97.1 (componente de rustup) | rustfmt |
| Swift | sourcekit-lsp de Xcode | swift-format |
| Go (`install go`) | gopls 0.24.0 con Go 1.27.3 | gofmt |

- **Go** se probó en un estado temporal, para no dejar sus 461 MB en el
  repositorio: `install go` tardó 27 s.
- **Fuera de `config/local/`** solo se añadió el componente rust-analyzer de
  rustup. `~/.cargo/bin/rust-analyzer` existía pero no funcionaba.

### Resultado

`scripts/lsp-check.py` abre sesiones reales de vkTERM (Zellij y Micro en un
PTY), una por lenguaje, en proyectos temporales: un símbolo definido en un archivo y usado en otro, un
error de tipos y código mal formateado.

| | TS | Python | Rust | Swift | Go |
|---|---|---|---|---|---|
| El servidor arranca solo al abrir un archivo | ✔ | ✔ | ✔ | ✔ | ✔ |
| Error: marca `>>` en el margen y mensaje abajo | ✔ | ✔ | ✔ (al guardar) | ✔ | ✔ |
| Información (hover) | ✔ | ✔ | ✔ | ✔ | ✔ |
| Ir a la definición (a otro archivo) | ✔ | ✔ | ✔ | ✔ | ✔ |
| Referencias | ✔ | — | — | — | — |
| Tab completa | ✔ | ✔ | — | — | ✔ |
| Formato | ✔ | ✔ | ✔ | ✔ | ✔ |
| Los servidores terminan al cerrar | ✔ | ✔ | ✔ | ✔ | ✔ |

"—": no se probó en ese lenguaje.

TypeScript, además:

- Un `.tsx` válido no muestra errores. Sin el parche, el mismo archivo da 8
  errores falsos ("'>' expected"…).
- Escribir 60 caracteres en un `.ts` de 3.000 líneas: se ven 1,4 s después de
  empezar a teclear, con 1,2 s de tecleo. No hay retraso apreciable.

Rust usa el `Cargo.toml` anidado de vkTERM (`plugins/vkterm/`). Sus errores de
tipos los da `cargo check` al guardar, como en VS Code.

**Pasada final** con la configuración definitiva: **41/41**
(`scripts/lsp-check.py`; Go con el estado temporal).

### Teclas en Ghostty real (sintetizadas)

| Tecla | Llega a la aplicación |
|---|---|
| F12 | ✔ `ESC [24~` |
| ⇧F12 | ✔ `ESC [24;2~` |
| F2 | ✔ `ESC O Q`, lo mismo que envía ⌘S (Guardar). En H1, Ghostty traducirá la tecla F2 física a otro código. |
| ⌃Space | ✘ **No llega.** En este Mac es el atajo de macOS "Seleccionar la fuente de entrada anterior" (activado, con dos distribuciones de teclado). Se quita del plan: las sugerencias serán con ⌘I, como en VS Code, y con Tab. |

### Problemas encontrados y resueltos

1. **typescript-language-server 6 no mandaba los errores.** Solo los envía a
   clientes que declaran `publishDiagnostics`, y mlsp no lo declara. Se
   comprobó con un cliente LSP mínimo: sin la capacidad, ningún diagnóstico;
   con ella, el error. Parche `03-publish-diagnostics.patch`.
2. **`.tsx`:** Micro los llama `typescript` y el servidor no entendía el JSX.
   Parche `02-jsx-language-id.patch` (`typescriptreact` / `javascriptreact`).
3. **Python sin información (hover):** a ruff se le quitaba con
   `hoverProvider = false`, pero mlsp solo mira si la capacidad existe y a
   veces le preguntaba a él. Ahora es `nil` (también falla en la
   configuración original de mlsp).
4. **rust-analyzer no encontraba el proyecto:** solo busca `Cargo.toml` en la
   raíz y un nivel por debajo. La configuración le pasa los que haya hasta
   tres niveles (`linkedProjects`) si no hay uno en la raíz.
5. **Micro no admite `#` con las listas de Go en Lua** (`__len undefined`):
   rompía la carga de mlsp ("Unknown command lsp"). Se convierten a texto con
   `strings.Join`.
6. **`node` de las sesiones:** al arrancar desde Ghostty (`zsh -l`, sin
   `.zshrc`) es el de Homebrew y no el de nvm. Cada servidor se lanza con la
   ruta absoluta del node con el que se instaló.
7. **npm reinstalaba cada vez:** se comparaba con el lockfile interno de npm.
   Ahora se guarda una copia propia (`.vkterm-lock`).

Comportamiento de Micro a tener en cuenta: el mensaje de un error sale abajo
cuando el cursor llega a la línea con una tecla (flechas, clic). Con el
comando `goto` no sale.

### Pendiente para H1

- Los atajos (F12, ⇧F12, ⌘K ⌘I, ⌘I, F2, ⌘K ⌘F), los comandos de la paleta y la
  categoría "Lenguaje" de la ventana de atajos. Hoy funciona con los comandos
  de Micro (Ctrl-E `lsp hover`, `lsp goto-definition`…), con Tab y con los
  errores, que salen solos.
- Renombrar: mlsp solo cambia los archivos abiertos (parche 4 del plan).
- `.jsx`: Micro no les da tipo de archivo y no arranca ningún servidor.
- `scripts/lsp-check.py` tarda varios minutos y va aparte de
  `smoke-test.py`. En H1 se decide qué parte entra en la suite.

### Suite (`smoke-test.py --ghostty`)

137/138. La parte PTY pasa entera. La de Ghostty no se pudo ejecutar: la
pantalla del Mac se bloqueó durante la pasada, y así macOS no crea ventanas
nuevas. H0 no cambia la configuración de Ghostty ni la de Zellij. La última
pasada con Ghostty, tras el hito P, fue 177/177.

### Limitaciones conocidas

- Micro no tiene listas desplegables: las sugerencias salen en la barra de
  estado y se recorren con Tab.
- Swift con `.xcodeproj` (vkBLINK, vkmonitor, vkrar): sourcekit-lsp solo ayuda
  archivo por archivo sin `xcode-build-server`.
- Solo se prueba con teclas sintetizadas. Con el teclado físico, F12 puede
  necesitar fn, según la configuración de macOS, igual que en VS Code.
