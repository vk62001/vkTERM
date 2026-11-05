#!/bin/zsh
# vkTERM — compila Ghostty con los parches de vkTERM (patches/ghostty). Con
# esa compilación, bin/vkterm-app install crea vkTERM.app en vez de copiar la
# Ghostty.app instalada.
#
#   scripts/build-ghostty.sh [VERSIÓN]   por defecto, la de /Applications/Ghostty.app
#   scripts/build-ghostty.sh --path [VERSIÓN]
#                                        dónde queda la app compilada (exista o no)
#
# El parche: Ghostty 1.3 dibuja sobre el terminal un indicador («⌨ vkterm»)
# mientras hay una tabla de atajos activa, es decir, siempre en el modo IDE, y
# no tiene opción para ocultarlo ni moverlo. La compilación de vkTERM no lo
# dibuja; el modo IDE se ve en la barra de estado del IDE
# (plugins/vkterm/src/status.rs). Las secuencias a medias (⌘K …) se siguen
# mostrando mientras se espera la segunda tecla.
#
# Hace falta Xcode 26 con su componente Metal Toolchain
# (xcodebuild -downloadComponent MetalToolchain) y red: se descargan Zig (la
# versión que pide Ghostty, comprobada con su SHA-256), el código de Ghostty
# (la etiqueta vVERSIÓN de GitHub) y sus dependencias; la primera compilación
# tarda unos minutos. Lo descargado y lo intermedio van a ~/Library/Caches/vkterm
# (se puede rehacer); la app compilada, a ~/Library/Application Support/vkterm:
# macOS vacía las cachés cuando falta espacio en el disco, y el 4 de noviembre
# de 2026 se llevó la compilación, con lo que bin/vkterm-app volvía en
# silencio a la Ghostty sin parches.
#
# Dos arreglos para las herramientas de Xcode 26, con un xcrun y un libtool
# propios delante en el PATH de la compilación:
#   - Zig 0.15 no enlaza con los SDK de macOS 26.4 y posteriores, cuyos .tbd
#     solo declaran arm64e (ziglang/zig#31665). Si es el caso, Zig recibe una
#     copia del SDK que también declara arm64; Xcode compila la app con el SDK
#     de siempre.
#   - Los .a de Zig tienen miembros sin alinear a 8 bytes, y el libtool de
#     Xcode 26 los descarta al juntarlos en libghostty-fat.a («not 8-byte
#     aligned»): faltaba libghostty_zcu.o y la app no enlazaba. El libtool
#     propio extrae los objetos de cada .a y se los pasa sueltos.
emulate -L zsh
setopt err_exit no_unset pipe_fail extended_glob

vkterm_home=${0:A:h:h}
cache=${VKTERM_BUILD_CACHE:-$HOME/Library/Caches/vkterm}
support=${VKTERM_SUPPORT_DIR:-$HOME/Library/Application Support/vkterm}
say() { print -r -- "build-ghostty: $*" }
fail() { print -u2 -r -- "build-ghostty: $*"; exit 1 }

# SHA-256 de los Zig que se saben descargar (https://ziglang.org/download/index.json).
typeset -A zig_sha=(
  0.15.2 3cc2bab367e185cdfb27501c4b30b1b0653c28d9f73df8dc91488e66ece5fa6b
)

path_only=0
if [[ ${1:-} == --path ]]; then
  path_only=1
  shift
fi
version=${1:-$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' \
  /Applications/Ghostty.app/Contents/Info.plist 2>/dev/null || true)}
[[ -n $version ]] || fail "no se encuentra /Applications/Ghostty.app: indica la versión (p. ej. 1.3.1)."
[[ $version == [0-9.]## ]] || fail "versión no válida: $version"
dir=$cache/ghostty-$version
target=$support/ghostty-$version/Ghostty.app
if (( path_only )); then
  print -r -- $target
  exit 0
fi
[[ $(uname -m) == arm64 ]] || fail "solo está preparado para Macs con Apple Silicon."

# Código de Ghostty, limpio y con los parches.
src=$dir/src
if [[ ! -d $src/.git ]]; then
  say "descargando Ghostty $version…"
  rm -rf $src
  mkdir -p $dir
  git -c advice.detachedHead=false clone --quiet --depth 1 --branch v$version https://github.com/ghostty-org/ghostty.git $src
fi
git -C $src reset --quiet --hard
for patch in $vkterm_home/patches/ghostty/*.patch(N); do
  git -C $src apply $patch || fail "el parche ${patch:t} no se aplica a Ghostty $version."
done

# Zig, la versión que pide Ghostty.
zig_version=$(sed -n 's/.*minimum_zig_version = "\(.*\)".*/\1/p' $src/build.zig.zon)
zig=$cache/zig-$zig_version/zig
if [[ ! -x $zig ]]; then
  sha=${zig_sha[$zig_version]:-}
  [[ -n $sha ]] || fail "Ghostty $version pide Zig $zig_version: añade su SHA-256 a zig_sha en este script."
  say "descargando Zig $zig_version…"
  tarball=$cache/zig-$zig_version.tar.xz
  curl -fsSL -o $tarball https://ziglang.org/download/$zig_version/zig-aarch64-macos-$zig_version.tar.xz
  print "$sha  $tarball" | shasum -a 256 -c --quiet || { rm -f $tarball; fail "la descarga de Zig no coincide con su SHA-256."; }
  rm -rf $cache/zig-$zig_version $cache/zig-aarch64-macos-$zig_version
  tar -xf $tarball -C $cache
  mv $cache/zig-aarch64-macos-$zig_version $cache/zig-$zig_version
  rm -f $tarball
fi

xcrun -sdk macosx metal --version >/dev/null 2>&1 \
  || fail "falta el Metal Toolchain de Xcode: xcodebuild -downloadComponent MetalToolchain"

tools=$cache/ghostty-tools
rm -rf $tools
mkdir -p $tools
print -r -- '#!/bin/zsh
# libtool -static -o SALIDA ENTRADAS…: cada .a, extraído en su carpeta.
[[ $1 == -static && $2 == -o ]] || exec /usr/bin/libtool "$@"
out=$3 tmp=$(mktemp -d) objs=() n=0
shift 3
for input; do
  if [[ $input == *.a ]]; then
    (( n += 1 ))
    mkdir $tmp/$n
    archive=${input:A}
    # Zig guarda los miembros con permisos 000.
    (cd $tmp/$n && /usr/bin/ar -x $archive && chmod 644 *) || { rm -rf $tmp; exit 1 }
    objs+=($tmp/$n/*.o(N))
  else
    objs+=($input)
  fi
done
/usr/bin/libtool -static -o $out $objs
code=$?
rm -rf $tmp
exit $code' > $tools/libtool
chmod +x $tools/libtool

# SDK para Zig: el de Xcode o, si sus .tbd no declaran arm64, una copia que sí.
sdk=${$(xcrun --sdk macosx --show-sdk-path):A}
if ! grep -q arm64-macos $sdk/usr/lib/system/libsystem_c.tbd; then
  sdk_copy=$cache/sdk-$(xcrun --sdk macosx --show-sdk-version)-arm64/MacOSX.sdk
  if [[ ! -d $sdk_copy ]]; then
    say "preparando una copia del SDK con arm64 para Zig…"
    rm -rf ${sdk_copy:h}
    mkdir -p ${sdk_copy:h}
    cp -R $sdk $sdk_copy.tmp
    python3 -I - $sdk_copy.tmp <<'PY'
import os, re, sys
lists = re.compile(r"\[[^\]]*\]")
for root, _, files in os.walk(sys.argv[1]):
    for name in files:
        path = os.path.join(root, name)
        if not name.endswith(".tbd") or os.path.islink(path):
            continue
        text = open(path, encoding="utf-8").read()
        new = lists.sub(lambda m: m.group(0) if "arm64-macos" in m.group(0)
                        else m.group(0).replace("arm64e-macos", "arm64-macos, arm64e-macos"), text)
        if new != text:
            open(path, "w", encoding="utf-8").write(new)
PY
    mv $sdk_copy.tmp $sdk_copy
  fi
  # Zig pregunta con «xcrun --sdk macosx --show-sdk-path»; el resto, al xcrun real.
  print -r -- "#!/bin/sh
case \"\$*\" in *--show-sdk-path*) echo '$sdk_copy'; exit 0 ;; esac
exec /usr/bin/xcrun \"\$@\"" > $tools/xcrun
  chmod +x $tools/xcrun
fi

say "compilando Ghostty $version con los parches de vkTERM (unos minutos)…"
(cd $src && PATH=$tools:$PATH $zig build -Doptimize=ReleaseFast -Dxcframework-target=native) \
  || fail "la compilación falló."
[[ -d $src/zig-out/Ghostty.app ]] || fail "la compilación no dejó zig-out/Ghostty.app."
mkdir -p ${target:h}
rm -rf $target.tmp
ditto $src/zig-out/Ghostty.app $target.tmp
# Xcode deja la versión de su proyecto (0.1): la de la etiqueta, la que mira
# bin/vkterm-app. La firma la rehace bin/vkterm-app al crear vkTERM.app.
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" \
  -c "Set :CFBundleVersion $version" $target.tmp/Contents/Info.plist
rm -rf $target
mv $target.tmp $target
say "listo: $target"
say "crea vkTERM.app con ella: bin/vkterm-app install (con vkTERM.app cerrada)."
