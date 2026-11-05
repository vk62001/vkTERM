#!/bin/zsh
# vkTERM — compila Zellij con los parches de vkTERM (patches/zellij). Con esa
# compilación, bin/vkterm-workspace la usa en vez del zellij del PATH.
#
#   scripts/build-zellij.sh [VERSIÓN]   por defecto, la del zellij del PATH
#   scripts/build-zellij.sh --path [VERSIÓN]
#                                       dónde queda el binario (exista o no)
#
# El parche: Zellij 0.45.1 solo pasa el clic derecho al panel que ya tiene el
# foco, y nunca a un plugin. En el IDE, el clic derecho no abría el menú del
# explorador (src/menu.rs) ni el del editor si el foco estaba en otro panel.
# Con el parche, el clic derecho enfoca el panel de debajo y llega a los
# plugins como Mouse::RightClick, como en VS Code. La rama principal de Zellij
# ya lo hace a su manera (MouseAction::ForwardRightClickToPlugin): el parche
# sobra cuando salga una versión con ese cambio.
#
# El segundo parche: si `zellij list-sessions` (lo ejecutan bin/vkterm-workspace
# y las pruebas) consulta una sesión que aún no ha recibido a su primer
# cliente, el servidor hacía unwrap() de una sesión que no existe y moría: la
# ventana del IDE se quedaba en blanco. Con esta compilación pasaba en 5 de 19
# arranques de las pruebas.
#
# Hace falta Rust (cargo) y red: se descargan el código de Zellij (la
# etiqueta vVERSIÓN de GitHub) y sus dependencias. Se compila con las mismas
# funciones que la fórmula de Homebrew. El binario queda en
# ~/Library/Application Support/vkterm y no en ~/Library/Caches: macOS vacía
# las cachés cuando falta espacio en el disco (así se perdieron el 4 de
# noviembre de 2026 la compilación de Ghostty y vkTERM Test.app). El código y
# la compilación intermedia sí van a la caché: se pueden rehacer.
emulate -L zsh
setopt err_exit no_unset pipe_fail extended_glob

vkterm_home=${0:A:h:h}
cache=${VKTERM_BUILD_CACHE:-$HOME/Library/Caches/vkterm}
support=${VKTERM_SUPPORT_DIR:-$HOME/Library/Application Support/vkterm}
say() { print -r -- "build-zellij: $*" }
fail() { print -u2 -r -- "build-zellij: $*"; exit 1 }

path_only=0
if [[ ${1:-} == --path ]]; then
  path_only=1
  shift
fi
version=${1:-$(zellij --version 2>/dev/null | awk '{print $2}')}
[[ -n $version ]] || fail "no se encuentra zellij en el PATH: indica la versión (p. ej. 0.45.1)."
[[ $version == [0-9.]## ]] || fail "versión no válida: $version"
target=$support/zellij-$version/zellij
if (( path_only )); then
  print -r -- $target
  exit 0
fi
(( $+commands[cargo] )) || fail "falta cargo (https://rustup.rs)."

src=$cache/zellij-$version/src
if [[ ! -d $src/.git ]]; then
  say "descargando Zellij $version…"
  rm -rf $src
  mkdir -p ${src:h}
  git -c advice.detachedHead=false clone --quiet --depth 1 --branch v$version https://github.com/zellij-org/zellij.git $src
fi
git -C $src reset --quiet --hard
for patch in $vkterm_home/patches/zellij/*.patch(N); do
  git -C $src apply $patch || fail "el parche ${patch:t} no se aplica a Zellij $version."
done

say "compilando (la primera vez tarda varios minutos)…"
# Como la fórmula de Homebrew: sin vendored_curl.
CARGO_TARGET_DIR=$cache/zellij-$version/target cargo build --release --locked \
  --manifest-path $src/Cargo.toml --no-default-features \
  --features plugins_from_target,web_server_capability -p zellij

built=$cache/zellij-$version/target/release/zellij
[[ $($built --version) == "zellij $version" ]] || fail "el binario compilado no dice ser zellij $version."
mkdir -p ${target:h}
cp $built $target.new
mv $target.new $target
say "listo: $target"
say "las sesiones nuevas del IDE lo usan; las abiertas siguen con el zellij con el que arrancaron."
