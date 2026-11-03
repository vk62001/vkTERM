# vkTERM — integración con zsh en el modo terminal de vkTERM.app (la carga
# config/zsh/.zshenv en el primer prompt, después del .zshrc del usuario).
#
#   vkterm             el comando de vkTERM (bin/vkterm), aunque no esté en
#                      el PATH: `vkterm .` convierte la pestaña en el IDE.
#   ⌘B ⌘J ⌘P ⌘⇧P      Ghostty envía F13–F16 con Super
#                      (config/ghostty/vkterm.ghostty). En el prompt
#                      convierten la pestaña en el IDE de la carpeta actual y
#                      después muestran el explorador, el Shell, la búsqueda de
#                      archivos o la paleta de comandos. Lo que se estaba
#                      escribiendo vuelve al prompt al cerrar la carpeta.
#   ⌘O                 Ghostty envía F20 con Super: en el prompt, «Abrir
#                      reciente» (`vkterm -r`), la lista de las carpetas ya
#                      abiertas en vkTERM, también desde la carpeta personal.
#   ⌘+ ⌘- ⌘0           Ghostty envía F17–F19 con Super: en el prompt
#                      cambian el tamaño de letra de vkTERM, que se recuerda
#                      (bin/vkterm-font), como en el IDE.
#   claude, codex      corren a través de bin/vkterm-agent: ⌘V con una imagen
#                      en el portapapeles la adjunta, como en el IDE. Si ya
#                      tienes una función con ese nombre, se respeta;
#                      `command claude` se salta el intermediario.
#   VKTERM_AUTOSTART   "nonce:acción:carpeta": la ventana se abre ya
#                      convertida (bin/vkterm-surface open). Si la variable
#                      llegó a varias pestañas, solo convierte la primera que
#                      reclama el nonce.
[[ -n ${VKTERM_HOME-} ]] || return 0

vkterm() { $VKTERM_HOME/bin/vkterm "$@" }
(( $+functions[claude] )) || claude() { $VKTERM_HOME/bin/vkterm-agent claude "$@" }
(( $+functions[codex] )) || codex() { $VKTERM_HOME/bin/vkterm-agent codex "$@" }

_vkterm_convert() {
  # Como un atajo no dice qué carpeta abrir, no se convierte ni la raíz ni la
  # carpeta personal; `vkterm ~` escrito a mano, sí.
  if [[ $PWD == / || $PWD == $HOME ]]; then
    zle -M "vkTERM: entra antes en la carpeta de un proyecto (cd …) o escribe vkterm RUTA"
    return
  fi
  zle push-line
  BUFFER="vkterm --then $1 ."
  zle accept-line
}
_vkterm_sidebar() { _vkterm_convert show_sidebar }
_vkterm_panel() { _vkterm_convert show_panel }
_vkterm_files() { _vkterm_convert quick_open }
_vkterm_commands() { _vkterm_convert command_palette }
_vkterm_recent() {
  zle push-line
  BUFFER="vkterm -r"
  zle accept-line
}
# Tamaño de letra: en segundo plano, para no esperar a la recarga de Ghostty.
_vkterm_font_size() {
  $VKTERM_HOME/bin/vkterm-font size $1 >/dev/null 2>&1 &!
}
_vkterm_font_bigger() { _vkterm_font_size +1 }
_vkterm_font_smaller() { _vkterm_font_size -1 }
_vkterm_font_reset() { _vkterm_font_size 0 }
zle -N _vkterm_sidebar
zle -N _vkterm_panel
zle -N _vkterm_files
zle -N _vkterm_commands
zle -N _vkterm_recent
zle -N _vkterm_font_bigger
zle -N _vkterm_font_smaller
zle -N _vkterm_font_reset
() {
  local keymap
  for keymap in emacs viins vicmd; do
    bindkey -M $keymap $'\e[25;9~' _vkterm_sidebar $'\e[26;9~' _vkterm_panel \
      $'\e[28;9~' _vkterm_files $'\e[29;9~' _vkterm_commands $'\e[34;9~' _vkterm_recent \
      $'\e[31;9~' _vkterm_font_bigger $'\e[32;9~' _vkterm_font_smaller $'\e[33;9~' _vkterm_font_reset
  done
}

if [[ -n ${VKTERM_AUTOSTART-} ]]; then
  typeset -g _vkterm_autostart=$VKTERM_AUTOSTART
  unset VKTERM_AUTOSTART
  _vkterm_autostart() {
    add-zle-hook-widget -d line-init _vkterm_autostart
    local nonce=${_vkterm_autostart%%:*} rest=${_vkterm_autostart#*:}
    local then=${rest%%:*} dir=${rest#*:}
    unset _vkterm_autostart
    mkdir ${TMPDIR:-/tmp}/vkterm-autostart-$nonce 2>/dev/null || return 0
    builtin cd -q -- $dir 2>/dev/null || return 0
    BUFFER="vkterm${then:+ --then $then} ."
    zle accept-line
  }
  autoload -Uz add-zle-hook-widget
  zle -N _vkterm_autostart
  add-zle-hook-widget line-init _vkterm_autostart
fi
