# Cambios

Los cambios de cada versión de vkTERM. El formato sigue
[Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y las versiones,
[SemVer](https://semver.org/lang/es/). La versión actual está en `VERSION`
(`vkterm --version`).

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
