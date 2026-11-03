#!/bin/zsh
# vkTERM — empaqueta vkTERM.app en un DMG para instalarla arrastrándola a
# Aplicaciones.
#
#   scripts/build-dmg.sh [carpeta]    por defecto, dist/ (fuera de git)
#
# Usa la app que creó bin/vkterm-app install (no la vuelve a crear) y falla si
# está desactualizada. El DMG se llama vkTERM-<VERSION>.dmg.
#
# La app no lleva vkTERM dentro, así que el DMG solo sirve en un Mac preparado:
#   - el lanzador busca el repositorio en la ruta de este Mac (clave
#     VKTermHome del Info.plist): en el otro tiene que estar en la misma;
#   - Zellij, Micro y el plugin compilado tienen que estar instalados
#     (scripts/doctor.sh);
#   - la firma es local (ad hoc): si el DMG llega descargado o por AirDrop,
#     Gatekeeper no abre la app hasta quitarle la cuarentena
#     (xattr -dr com.apple.quarantine /Applications/vkTERM.app).
# En otro Mac, lo fiable sigue siendo clonar el repositorio y ejecutar
# bin/vkterm-app install allí.
emulate -L zsh
setopt err_exit no_unset pipe_fail

vkterm_home=${0:A:h:h}
version=$(<$vkterm_home/VERSION)
dmg=${${1:-$vkterm_home/dist}:A}/vkTERM-$version.dmg

say() { print -r -- "build-dmg: $*" }
fail() { print -u2 -r -- "build-dmg: $*"; exit 1 }

app=$($vkterm_home/bin/vkterm-app path)
[[ -d $app ]] || fail "falta vkTERM.app: ejecuta bin/vkterm-app install."
$vkterm_home/bin/vkterm-app status >/dev/null \
  || fail "vkTERM.app está desactualizada: ejecuta bin/vkterm-app install."
codesign --verify --deep --strict $app \
  || fail "la firma de $app no es válida: ejecuta bin/vkterm-app install --force."

staging=$(mktemp -d -t vkterm-dmg)
trap 'rm -rf $staging' EXIT
ditto $app $staging/vkTERM.app
ln -s /Applications $staging/Applications

mkdir -p ${dmg:h}
say "creando ${dmg:t} con $app…"
hdiutil create -quiet -ov -volname "vkTERM $version" -srcfolder $staging \
  -fs HFS+ -format UDZO $dmg
hdiutil verify -quiet $dmg
say "$dmg ($(du -h $dmg | awk '{print $1}'))"
say "sha256 $(shasum -a 256 $dmg | cut -d' ' -f1)"
