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
| [Ghostty](https://github.com/ghostty-org/ghostty) 1.3.1 | Terminal gráfica | MIT | `/Applications/Ghostty.app` |
| [Codex CLI](https://github.com/openai/codex) | Proceso CLI en un panel (no se distribuye) | Apache-2.0 | Instalación del usuario |
| Claude Code | Proceso CLI en un panel (no se distribuye) | Propietaria (términos de Anthropic) | Instalación del usuario |

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
- Los temas del selector (⌘K ⌘T) que traen Zellij y Ghostty (Catppuccin,
  Dracula, Nord, Tokyo Night, Gruvbox, Dark Modern) se referencian por nombre;
  los proporcionan esas aplicaciones y no se copian aquí.
- `config/ghostty/themes/Light Modern` y `config/zellij/themes/vkterm.kdl`:
  colores de los temas "Dark Modern" y "Light Modern" de
  [Visual Studio Code](https://github.com/microsoft/vscode) (editor y colores
  ANSI del terminal). Copyright (c) Microsoft Corporation. Licencia MIT.
- `plugins/vkterm/src/themes.rs`: los colores del atenuado se eligieron a
  partir de las paletas de esos temas (p. ej. overlay0 y crust de Catppuccin,
  MIT).

Cuando se haga un fork de Zellij o Ghostty, o se empaqueten binarios, habrá que
incluir sus textos de licencia completos junto a la distribución.
