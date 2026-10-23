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
row vkTERM "" "$(<${0:A:h:h}/VERSION)"
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
  row vkterm.wasm ok "compilado (vkterm lo recompila si cambian las fuentes)"
else
  row vkterm.wasm pendiente "se compila al abrir el IDE (scripts/build-plugins.sh)"
fi

print "\nvkTERM.app (la app del Dock: bin/vkterm-app install)"
ghostty_version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' \
  ${VKTERM_GHOSTTY_APP:-/Applications/Ghostty.app}/Contents/Info.plist 2>/dev/null || true)
if [[ -z $ghostty_version ]]; then
  row Ghostty FALTA "vkTERM.app es una copia de Ghostty 1.3 o posterior"
elif [[ $(printf '%s\n%s\n' 1.3.0 $ghostty_version | sort -V | head -1) != 1.3.0 ]]; then
  row Ghostty viejo "$ghostty_version: hace falta 1.3 (tablas de atajos y AppleScript)"
fi
${0:A:h:h}/bin/vkterm-app status | sed 's/^/  /'
link=$HOME/.local/bin/vkterm
if [[ -L $link ]]; then
  row vkterm ok "$link → $(readlink $link)"
else
  row vkterm "sin enlace" "bin/vkterm-app install --link (o usa bin/vkterm)"
fi

print "\nLenguajes (LSP, opcional: bin/vkterm-plugins install)"
${0:A:h:h}/bin/vkterm-plugins status | sed 's/^vkterm-plugins: /  /'

exit $missing
