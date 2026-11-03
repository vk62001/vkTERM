# Avisos de terceros

vkTERM **no redistribuye binarios ni código fuente de terceros**: usa las
versiones instaladas en el sistema y el repositorio solo contiene configuración
y código propios. El plugin `plugins/vkterm` se compila en cada equipo (el
`.wasm` no se versiona). Esta tabla registra de qué depende y bajo qué
licencia, para cuando se haga un fork o se empaquete una distribución.

| Componente | Uso en vkTERM | Licencia | Origen |
|---|---|---|---|
| [Zellij](https://github.com/zellij-org/zellij) 0.45.1 | Gestor de paneles, sesiones y plugins | MIT | Homebrew (`zellij`) |
| [zellij-tile](https://crates.io/crates/zellij-tile) 0.45.1 y zellij-utils | API de plugins; se compila dentro de `vkterm.wasm` | MIT | crates.io (`plugins/vkterm/Cargo.lock`) |
| [Micro](https://github.com/micro-editor/micro) 2.0.15 | Editor | MIT | Homebrew (`micro`) |
| [Ghostty](https://github.com/ghostty-org/ghostty) 1.3.1 | Terminal gráfica. vkTERM.app es una copia local de la Ghostty.app instalada (otro nombre, icono e identificador; el mismo binario) o de Ghostty compilado en el equipo con el parche de vkTERM (`patches/ghostty`, `scripts/build-ghostty.sh`). La crea `bin/vkterm-app` en cada equipo y no se versiona | MIT | `/Applications/Ghostty.app` o la etiqueta `v1.3.1` de GitHub |
| [Zig](https://ziglang.org) 0.15.2 | Solo para compilar Ghostty (`scripts/build-ghostty.sh`); no se distribuye | MIT | ziglang.org, comprobado con su SHA-256 |
| [Codex CLI](https://github.com/openai/codex) | Proceso CLI en un panel (no se distribuye) | Apache-2.0 | Instalación del usuario |
| Claude Code | Proceso CLI en un panel (no se distribuye) | Propietaria (términos de Anthropic) | Instalación del usuario |

### Lenguajes (LSP): los descarga `bin/vkterm-plugins install`

No están en el repositorio: el comando los descarga con versiones fijas
(`config/lsp/`) en el estado local, `config/local/`, que no se versiona.

| Componente | Uso en vkTERM | Licencia | Origen |
|---|---|---|---|
| [µlsp (mlsp)](https://github.com/Andriamanitra/mlsp) `91261a0` | Plugin LSP de Micro | MIT | GitHub (git, commit fijo) |
| [typescript-language-server](https://github.com/typescript-language-server/typescript-language-server) 6.0.2 | Servidor de TypeScript / JavaScript | Apache-2.0 | npm (`config/lsp/package-lock.json`) |
| [TypeScript](https://github.com/microsoft/TypeScript) 6.0.3 | tsserver | Apache-2.0 | npm |
| [Pyright](https://github.com/microsoft/pyright) 1.1.414 | Servidor de Python | MIT | npm |
| [Ruff](https://github.com/astral-sh/ruff) 0.16.10 | Formato y avisos de Python | MIT | GitHub releases (sha256 fijo) |
| [rust-analyzer](https://github.com/rust-lang/rust-analyzer) | Servidor de Rust | MIT o Apache-2.0 | `rustup component add rust-analyzer` |
| [sourcekit-lsp](https://github.com/swiftlang/sourcekit-lsp) y swift-format | Servidor y formato de Swift | Apache-2.0 con excepción de runtime | Xcode |
| [Go](https://go.dev) 1.27.3 y [gopls](https://github.com/golang/tools/tree/master/gopls) 0.24.0 | Servidor de Go (solo con `install go`) | BSD-3-Clause | go.dev (sha256 fijo) y proxy.golang.org |

### Dependencias del plugin (`plugins/vkterm/Cargo.lock`)

Además de zellij-tile, el `.wasm` incluye 131 crates transitivas. Sus licencias
(campo `license` de `cargo metadata`) son MIT y/o Apache-2.0 en su mayoría,
Unicode-3.0 (crates ICU4X), ISC, Zlib, 0BSD y Unlicense (en alternativa con
MIT), y **MPL-2.0** en dos crates sin modificar (`colored`, `option-ext`). Si
se distribuye el `.wasm`, hay que acompañarlo de esos textos de licencia (por
ejemplo, generados con `cargo about`) e indicar dónde obtener el código de las
crates MPL-2.0.

## Material derivado incluido en este repositorio

- `config/zellij/config.kdl`: los atajos derivan del preset
  "Unlock-First (non-colliding)" de Zellij
  (`default-plugins/configuration/src/presets.rs`, v0.45.1).
  Copyright (c) 2020 Zellij contributors. Licencia MIT.
- Temas de color (⌘K ⌘T): `config/themes/vscode/` guarda copias sin cambios
  de temas de VS Code, con su licencia en cada carpeta y su procedencia y
  sha256 en `SOURCES.json`. `scripts/gen-themes.py` genera a partir de ellos
  los temas de Ghostty (`config/ghostty/themes/vkterm-*`), Micro
  (`config/micro/colorschemes/vkterm-*.micro`), Zellij
  (`config/zellij/themes/vkterm.kdl`) y la tabla del plugin. Todos con
  licencia MIT:
  - Dark Modern, Light Modern, Dark+, Light+, Dark y Light High Contrast,
    Monokai, Solarized Dark y Light, Abyss, Quiet Light, Kimbie Dark y
    Tomorrow Night Blue, de [Visual Studio Code](https://github.com/microsoft/vscode)
    1.143.1, y los valores por defecto de su registro de colores
    (`colorRegistry`, `terminalColorRegistry.ts`), copiados en
    `scripts/gen-themes.py`. Copyright (c) 2015 - present Microsoft Corporation.
  - [Catppuccin](https://github.com/catppuccin/vscode) 3.19.0 (Mocha y Latte).
    Copyright (c) 2021 Catppuccin.
  - [Dracula](https://github.com/dracula/visual-studio-code) 2.25.1.
    Copyright (c) 2016 Dracula Theme.
  - [Nord](https://github.com/nordtheme/visual-studio-code) 0.19.0.
    Copyright (C) 2017-present Arctic Ice Studio.
  - [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme) 1.1.2.
    Copyright (c) 2018-present Enkia.
  - [Gruvbox](https://github.com/jdinhify/vscode-theme-gruvbox) 1.29.1
    (Dark y Light Medium). Copyright © 2017 JD.
  - [SynthWave '84](https://github.com/robb0wen/synthwave-vscode) 0.1.20.
    Copyright (c) 2019 Robb Owen.
  - [Shades of Purple](https://github.com/ahmadawais/shades-of-purple-vscode)
    7.3.6. Copyright (c) 2015 Ahmad Awais.
  - [Cobalt2](https://github.com/wesbos/cobalt2-vscode) 2.4.3.
    Copyright (c) 2018 Wes Bos, Roberto Achar.
- `config/themes/vscode/vkterm/monokai-pro.json`: Monokai Pro es un tema de
  pago y no se copia. Este archivo es de vkTERM y usa solo la paleta de
  terminal «Monokai Pro» que trae Ghostty (de
  [iTerm2-Color-Schemes](https://github.com/mbadolato/iTerm2-Color-Schemes),
  MIT); es una aproximación, no el tema original.
- El tema predeterminado de Ghostty hasta que se elige uno («Catppuccin
  Mocha») se referencia por nombre: lo trae Ghostty.
- `config/lsp/mlsp.lua`: adaptado del `config.lua` de
  [µlsp](https://github.com/Andriamanitra/mlsp), y `config/lsp/patches/`:
  cambios sobre su `main.lua` (`01-format-on-save.patch` es la PR #72 del
  propio proyecto). Copyright (c) 2023 Andriamanitra. Licencia MIT.
- `config/zsh/.zshenv`: el encadenado de ZDOTDIR sigue el de la integración
  de zsh de Ghostty (`shell-integration/zsh/.zshenv`, basada a su vez en la
  de kitty, GPL-3.0), compatible con la licencia de vkTERM.
- `config/themes/themes.json`: los colores del atenuado de los nueve primeros
  temas se eligieron a partir de sus paletas (p. ej. overlay0 y crust de
  Catppuccin, MIT); los del resto se calculan con el fondo y el texto.

Cuando se haga un fork de Zellij o Ghostty, o se empaqueten binarios, habrá que
incluir sus textos de licencia completos junto a la distribución. Un DMG
(`scripts/build-dmg.sh`) de una vkTERM.app creada con Ghostty compilado lleva
un Ghostty modificado: la licencia MIT lo permite, con su texto de licencia.
