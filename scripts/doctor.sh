#!/bin/zsh
# vkTERM — auditoría del entorno. Solo lee; no instala ni modifica nada.
#
#   scripts/doctor.sh
#
# Sale con código 1 si falta alguna herramienta imprescindible.
emulate -L zsh
setopt no_unset pipe_fail

missing=0

row() { printf '  %-12s %-10s %s\n' "$1" "$2" "$3" }

check() {
  local name=$1 required=$2 bin=$3; shift 3
  local version
  if [[ -x $bin ]] || (( $+commands[$bin] )); then
    version=$("$@" 2>/dev/null | head -1)
    row "$name" ok "$version"
  else
    row "$name" FALTA "(necesario: $required)"
    [[ $required == sí ]] && missing=1
  fi
}

print "Sistema"
row macOS "" "$(sw_vers -productVersion) ($(uname -m))"
print "\nHerramientas"
check Homebrew   no  brew   brew --version
check Git        sí  git    git --version
check Zellij     sí  zellij zellij --version
check Micro      sí  micro  micro -version
check jq         sí  jq     jq --version   # bin/vkterm-open (incluido en macOS 15+)
check Rust       sí  rustc  rustc --version   # compila el plugin vkterm
check Cargo      sí  cargo  cargo --version
check "Claude Code" no claude claude --version
check Codex      no  codex  codex --version

ghostty=${VKTERM_GHOSTTY_APP:-/Applications/Ghostty.app}/Contents/MacOS/ghostty
check Ghostty    no  $ghostty $ghostty --version

print "\nPlugin vkterm (Rust → WebAssembly)"
if (( $+commands[rustup] )) && rustup target list --installed 2>/dev/null | grep -qx wasm32-wasip1; then
  row wasm32-wasip1 ok "target instalado"
else
  row wasm32-wasip1 FALTA "rustup target add wasm32-wasip1"
  missing=1
fi
wasm=${0:A:h:h}/plugins/vkterm/target/wasm32-wasip1/release/vkterm.wasm
if [[ -f $wasm ]]; then
  row vkterm.wasm ok "compilado (bin/vkterm lo recompila si cambian las fuentes)"
else
  row vkterm.wasm pendiente "se compila al arrancar bin/vkterm (scripts/build-plugins.sh)"
fi

print "\nLenguajes (LSP, opcional: bin/vkterm-plugins install)"
${0:A:h:h}/bin/vkterm-plugins status | sed 's/^vkterm-plugins: /  /'

exit $missing
