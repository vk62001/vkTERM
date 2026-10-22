#!/bin/zsh
# vkTERM — compila los plugins de Zellij (Rust → wasm32-wasip1).
#
#   scripts/build-plugins.sh [--if-needed] [--test]
#
#   --if-needed  solo compila si falta el .wasm o hay fuentes más nuevas
#                (bin/vkterm lo usa al arrancar).
#   --test       además, ejecuta los tests de la lógica pura en el Mac.
emulate -L zsh
setopt err_exit no_unset pipe_fail

plugin=${0:A:h:h}/plugins/vkterm
wasm=$plugin/target/wasm32-wasip1/release/vkterm.wasm

if [[ ${1:-} == --if-needed && -f $wasm ]]; then
  stale=0
  for src in $plugin/Cargo.toml $plugin/src/**/*.rs; do
    if [[ $src -nt $wasm ]]; then
      stale=1
    fi
  done
  (( stale )) || exit 0
fi

if (( ! $+commands[cargo] )); then
  print -u2 "vkterm: hace falta Rust (cargo) para compilar el plugin (ver scripts/doctor.sh)."
  exit 1
fi
if ! rustup target list --installed 2>/dev/null | grep -qx wasm32-wasip1; then
  print -u2 "vkterm: falta el target wasm32-wasip1: rustup target add wasm32-wasip1"
  exit 1
fi

print -u2 "vkterm: compilando el plugin de Zellij…"
(cd $plugin && cargo build --release --quiet)
if [[ ${1:-} == --test || ${2:-} == --test ]]; then
  (cd $plugin && cargo test --lib --quiet --target "$(rustc -vV | sed -n 's/^host: //p')")
fi
