// vkTERM — ejecutable principal de vkTERM.app (lo compila bin/vkterm-app).
//
// vkTERM.app es una copia local de Ghostty.app con su propio identificador,
// nombre e icono. macOS la lanza sin argumentos (Dock, Finder, `open`), así
// que este programa hace de puente: sustituye su proceso (execv) por el
// `ghostty` real del bundle con el perfil de vkTERM y sin la configuración
// global del usuario, igual que hacía bin/vkterm-ghostty.
//
// Además exporta a Ghostty (y a los shells que abra):
//   VKTERM_HOME          repositorio de vkTERM (clave VKTermHome del Info.plist)
//   VKTERM_APP_ID        identificador de este bundle, para AppleScript
//   ZDOTDIR              config/zsh: integración de vkTERM con zsh; el
//                        .zshenv de esa carpeta devuelve el ZDOTDIR original
//   VKTERM_USER_ZDOTDIR  ese ZDOTDIR original, si había uno
#include <CoreFoundation/CoreFoundation.h>
#include <libgen.h>
#include <limits.h>
#include <mach-o/dyld.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static char *info_string(CFStringRef key) {
  CFTypeRef value = CFBundleGetValueForInfoDictionaryKey(CFBundleGetMainBundle(), key);
  char buffer[PATH_MAX];
  if (!value || CFGetTypeID(value) != CFStringGetTypeID()
      || !CFStringGetCString(value, buffer, sizeof buffer, kCFStringEncodingUTF8))
    return NULL;
  return strdup(buffer);
}

int main(int argc, char **argv) {
  char exe[PATH_MAX], real[PATH_MAX], ghostty[PATH_MAX];
  char profile[PATH_MAX + 32], zdotdir[PATH_MAX];
  uint32_t size = sizeof exe;
  if (_NSGetExecutablePath(exe, &size) != 0 || !realpath(exe, real)) {
    fprintf(stderr, "vkTERM: no se encuentra el propio ejecutable\n");
    return 1;
  }
  snprintf(ghostty, sizeof ghostty, "%s/ghostty", dirname(real));
  char *home = info_string(CFSTR("VKTermHome"));
  char *id = info_string(kCFBundleIdentifierKey);
  if (!home || !id) {
    fprintf(stderr, "vkTERM: al Info.plist le falta VKTermHome; vuelve a ejecutar bin/vkterm-app install\n");
    return 1;
  }
  setenv("VKTERM_HOME", home, 1);
  setenv("VKTERM_APP_ID", id, 1);
  snprintf(zdotdir, sizeof zdotdir, "%s/config/zsh", home);
  const char *user_zdotdir = getenv("ZDOTDIR");
  if (user_zdotdir && *user_zdotdir && strcmp(user_zdotdir, zdotdir) != 0)
    setenv("VKTERM_USER_ZDOTDIR", user_zdotdir, 1);
  setenv("ZDOTDIR", zdotdir, 1);
  snprintf(profile, sizeof profile, "--config-file=%s/config/ghostty/vkterm.ghostty", home);

  char **args = calloc(argc + 3, sizeof *args);
  if (!args) return 1;
  args[0] = ghostty;
  args[1] = "--config-default-files=false";
  args[2] = profile;
  for (int i = 1; i < argc; i++) args[i + 2] = argv[i];
  execv(ghostty, args);
  perror("vkTERM: execv");
  return 1;
}
