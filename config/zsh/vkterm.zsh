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
#   VKTERM_AUTOSTART   "nonce:acción:carpeta": la ventana se abre ya
#                      convertida (bin/vkterm-surface open). Si la variable
#                      llegó a varias pestañas, solo convierte la primera que
#                      reclama el nonce.
[[ -n ${VKTERM_HOME-} ]] || return 0

vkterm() { $VKTERM_HOME/bin/vkterm "$@" }

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
zle -N _vkterm_sidebar
zle -N _vkterm_panel
zle -N _vkterm_files
zle -N _vkterm_commands
() {
  local keymap
  for keymap in emacs viins vicmd; do
    bindkey -M $keymap $'\e[25;9~' _vkterm_sidebar $'\e[26;9~' _vkterm_panel \
      $'\e[28;9~' _vkterm_files $'\e[29;9~' _vkterm_commands
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
