# vkTERM — puente de zsh para las pestañas de vkTERM.app.
#
# El lanzador de la app (scripts/vkterm-app-launcher.c) pone ZDOTDIR en esta
# carpeta, así que zsh lee este archivo en lugar del .zshenv del usuario. La
# integración de Ghostty hace lo mismo y encadena: deja aquí ZDOTDIR antes de
# cargarlo. Este archivo:
#   1. devuelve ZDOTDIR a su valor original (VKTERM_USER_ZDOTDIR o nada), para
#      que zsh lea después los .zprofile, .zshrc y .zlogin del usuario;
#   2. carga el .zshenv del usuario;
#   3. en un shell interactivo, carga vkterm.zsh en el primer prompt, ya
#      después del .zshrc del usuario (sus bindkey no pisan los de vkTERM).
if [[ -n ${VKTERM_USER_ZDOTDIR-} ]]; then
  export ZDOTDIR=$VKTERM_USER_ZDOTDIR
else
  unset ZDOTDIR
fi
unset VKTERM_USER_ZDOTDIR
typeset -g _vkterm_zsh=${${(%):-%x}:A:h}/vkterm.zsh
[[ ! -r ${ZDOTDIR-$HOME}/.zshenv ]] || source ${ZDOTDIR-$HOME}/.zshenv
if [[ -o interactive ]]; then
  _vkterm_first_prompt() {
    precmd_functions=(${precmd_functions:#_vkterm_first_prompt})
    unfunction _vkterm_first_prompt
    [[ ! -r $_vkterm_zsh ]] || source $_vkterm_zsh
    unset _vkterm_zsh
  }
  typeset -ga precmd_functions
  precmd_functions+=(_vkterm_first_prompt)
else
  unset _vkterm_zsh
fi
