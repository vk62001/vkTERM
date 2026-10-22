# Próximos pasos: GitHub y plugins

Estado al 21-10-2026 (noche). Lo hecho esta noche está en
[phase-1-report.md](phase-1-report.md), secciones 10 a 12:

- ventana de atajos (⌘K ⌘S);
- teclado normal, en el que los dos ⌥ escriben caracteres;
- temas (⌘K ⌘T).

Este documento recoge lo que falta decidir.

## 1. Verificar primero (la pantalla estaba bloqueada)

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

### A. Paleta de comandos (⌘⇧P) y ⌘P con búsqueda

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
