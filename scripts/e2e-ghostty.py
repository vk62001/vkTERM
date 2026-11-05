#!/usr/bin/env python3
"""vkTERM — prueba de extremo a extremo en vkTERM Test.app, con varias ventanas.

    scripts/e2e-ghostty.py [--windows N]     (N ventanas, 4 por defecto)

Comprueba lo que ve el usuario, no la configuración: abre N ventanas en
vkTERM Test.app (N-1 convertidas en el IDE de carpetas distintas y una en el
modo terminal), cambia de una a otra varias veces y en cada una mira si los
atajos llegan a esa ventana y no a otra, si copiar deja el texto en el
portapapeles de macOS, si pegar llega al Shell, si la selección con el ratón
se copia, si el clic derecho abre el menú (en el explorador y en el editor sin
el foco), el TERM que ve un servidor por ssh y si el modo IDE sobrevive a una
recarga de la configuración de Ghostty.

Todo entra por la AppleScript de Ghostty 1.3 (`send key`, `send mouse …`,
`perform action`, `input text`), que no necesita el permiso de Accesibilidad
y no le roba el teclado ni el ratón al usuario. Límite: `send key` manda la
tecla física sin carácter, así que solo se pueden pulsar los atajos que van
por tecla física (⌘1–⌘5, ⌘⌥←/→, ⌃Tab…). Los que van por carácter (⌘C, ⌘V,
⌘K …) se prueban con la misma acción de Ghostty que ejecuta el atajo
(`perform action`): se marcan «acción del atajo». Con Accesibilidad para el
proceso que la lanza (iTerm, p. ej.), scripts/smoke-test.py --ghostty pulsa
también esas teclas.

El portapapeles del usuario se guarda en memoria (scripts/pasteboard.swift) y
se devuelve al final si nadie copió nada mientras tanto. La prueba nunca
escribe su contenido. Necesita que vkTERM Test.app esté cerrada; la primera
vez macOS pregunta si vkTERM Test puede leer el disco del repositorio y si el
terminal que la lanza puede controlarla (AppleScript).
"""
import importlib.util
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
_spec = importlib.util.spec_from_file_location("smoke", ROOT / "scripts" / "smoke-test.py")
smoke = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(smoke)
check, wait_until, run, Session, pane_id = smoke.check, smoke.wait_until, smoke.run, smoke.Session, smoke.pane_id
APP_ID = smoke.TEST_APP_ID
TITLES = smoke.TITLES
MENU_TITLE = smoke.MENU_TITLE
SHORTCUTS_TITLE = smoke.SHORTCUTS_TITLE

# Un programa que activa el ratón (SGR) y apunta cada evento: con dos clics se
# sabe qué celda corresponde a cada punto de la superficie de Ghostty.
MOUSE_PROBE = r"""
import os, sys, termios, tty
fd = sys.stdin.fileno(); old = termios.tcgetattr(fd); tty.setraw(fd)
sys.stdout.write("\x1b[?1000h\x1b[?1006h"); sys.stdout.flush()
log = open(sys.argv[1], "a", buffering=1); buf = b""
try:
    while True:
        buf += os.read(fd, 64)
        if b"q" in buf: break
        while b"M" in buf or b"m" in buf:
            end = min(i for i in (buf.find(b"M"), buf.find(b"m")) if i >= 0)
            log.write(buf[:end + 1].decode(errors="replace").split("<")[-1] + "\n"); buf = buf[end + 1:]
finally:
    sys.stdout.write("\x1b[?1006l\x1b[?1000l"); sys.stdout.flush()
    termios.tcsetattr(fd, termios.TCSADRAIN, old)
"""


def tell(script):
    return smoke.osa('tell application id "%s"\n%s\nend tell' % (APP_ID, script))


def key(terminal, name, mods="command"):
    tell('send key "%s"%s to terminal id "%s"' % (name, ' modifiers "%s"' % mods if mods else "", terminal))
    time.sleep(0.5)


def action(terminal, *names):
    for name in names:
        tell('perform action "%s" on terminal id "%s"' % (name, terminal))
    time.sleep(0.6)


def mouse(terminal, x, y, button=None, mods=None):
    """Mueve el puntero a (x, y) (puntos de la superficie) y, si se pide, pulsa y suelta."""
    m = ' modifiers "%s"' % mods if mods else ""
    tell('send mouse position x %s y %s%s to terminal id "%s"' % (x, y, m, terminal))
    if button:
        for act in ("press", "release"):
            tell('send mouse button %s button action %s%s to terminal id "%s"' % (button, act, m, terminal))
    time.sleep(0.6)


def drag(terminal, start, end):
    tell('send mouse position x %s y %s to terminal id "%s"' % (*start, terminal))
    tell('send mouse button left button action press to terminal id "%s"' % terminal)
    for i in range(1, 6):
        x = start[0] + (end[0] - start[0]) * i / 5
        tell('send mouse position x %s y %s to terminal id "%s"' % (x, end[1], terminal))
    tell('send mouse button left button action release to terminal id "%s"' % terminal)
    time.sleep(1.0)


def new_window():
    return tell("set w to new window\nreturn id of focused terminal of selected tab of w")


def focus(terminal):
    tell('focus terminal id "%s"' % terminal)
    time.sleep(0.6)


class Pasteboard:
    """scripts/pasteboard.swift: el del usuario queda guardado en memoria."""

    def __init__(self, binary):
        self.proc = subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.ours = self.ask("count")

    def ask(self, line):
        self.proc.stdin.write(line + "\n")
        self.proc.stdin.flush()
        return self.proc.stdout.readline().strip()

    def set(self, text):
        self.ours = self.ask("set " + text)

    def has(self, text):
        found = self.ask("has " + text) == "yes"
        if found:
            self.ours = self.ask("count")
        return found

    def restore(self):
        result = self.ask("restore " + self.ours)
        self.proc.stdin.close()
        self.proc.wait(timeout=5)
        return result


class Window:
    """Una ventana de vkTERM Test.app; `s` es su sesión de Zellij si es un IDE."""

    def __init__(self, n, terminal, project=None, s=None):
        self.n, self.terminal, self.project, self.s = n, terminal, project, s
        self.cell = None  # (ancho, alto, x0, y0) de una celda, en puntos

    @property
    def label(self):
        return "ventana %d (%s)" % (self.n, self.project.name if self.project else "modo terminal")


def shell_run(w, command, marker, timeout=8):
    """Ejecuta `command` en el Shell del IDE (zellij write-chars) y devuelve la línea con `marker`."""
    shell = w.s.pane("Shell")
    w.s.action("write-chars", "-p", pane_id(shell), command + "\n")
    return wait_until(lambda: next((l for l in w.s.screen("Shell").splitlines()
                                    if marker in l and "echo" not in l), None), timeout=timeout)


def calibrate(w, probe, tmp):
    """Celda ↔ punto: dos clics sobre un programa con el ratón activado en el
    Shell, ampliado (⌘K Z) para que los clics caigan en él sea cual sea el
    tamaño de la ventana."""
    log = tmp / ("mouse-%d.log" % w.n)
    log.write_text("")
    key(w.terminal, "digit5")
    action(w.terminal, "esc:z")
    time.sleep(0.8)
    shell = w.s.pane("Shell")
    w.s.action("write-chars", "-p", pane_id(shell), "clear; python3 %s %s\n" % (probe, log))
    time.sleep(1.5)
    points, cells = [(120.0, 100.0), (300.0, 200.0)], []
    for x, y in points:
        before = len(log.read_text().splitlines())
        mouse(w.terminal, x, y, "left")
        line = wait_until(lambda: (lambda ls: ls[before] if len(ls) > before else None)(log.read_text().splitlines()))
        if not line:
            break
        _, col, row = (int(v) for v in line.rstrip("Mm").split(";"))
        cells.append((col - 1 + shell["pane_content_x"], row - 1 + shell["pane_content_y"]))
    w.s.action("write-chars", "-p", pane_id(shell), "q")
    time.sleep(0.5)
    action(w.terminal, "esc:z")
    time.sleep(0.8)
    if len(cells) != 2 or cells[0][0] == cells[1][0] or cells[0][1] == cells[1][1]:
        return None
    cw = (points[1][0] - points[0][0]) / (cells[1][0] - cells[0][0])
    ch = (points[1][1] - points[0][1]) / (cells[1][1] - cells[0][1])
    # Origen de la columna 0 / fila 0 (el borde izquierdo / superior de la celda).
    x0 = points[0][0] - cells[0][0] * cw - cw / 2
    y0 = points[0][1] - cells[0][1] * ch - ch / 2
    w.cell = (cw, ch, x0, y0)
    return w.cell


def point(w, col, row):
    cw, ch, x0, y0 = w.cell
    return x0 + (col + 0.5) * cw, y0 + (row + 0.5) * ch


def probe_focus(w):
    """⌘2 y ⌘1 (tecla física) en esta ventana: ¿llegan a su IDE?"""
    key(w.terminal, "digit2")
    editor = wait_until(lambda: w.s.focused() == "Editor", timeout=3)
    key(w.terminal, "digit1")
    explorer = wait_until(lambda: w.s.focused() == "Explorador", timeout=3)
    return bool(editor and explorer)


def ide_checks(w, windows, board, tmp, probe):
    print("\n%s" % w.label.capitalize())
    s = w.s
    focus(w.terminal)
    others = {o.n: o.s.focused() for o in windows if o.s and o is not w}
    check("%s: ⌘2 / ⌘1 / ⌘5 (tecla física) mueven el foco de su IDE" % w.label,
          probe_focus(w) and (key(w.terminal, "digit5") or wait_until(lambda: s.focused() == "Shell", timeout=3)),
          "foco: %s" % s.focused(), s)
    check("%s: las otras ventanas no reciben esas teclas" % w.label,
          all(o.s.focused() == others[o.n] for o in windows if o.s and o is not w),
          str({o.n: o.s.focused() for o in windows if o.s and o is not w}))

    # ssh: lo que ve un servidor sin el terminfo de Ghostty (como un VPS).
    line = shell_run(w, "echo E2E-TERM=$TERM; (env -u TERMINFO TERMINFO_DIRS=/usr/share/terminfo HOME=/var/empty "
                        "tput clear >/dev/null 2>&1 && echo E2E-REMOTO=ok) || echo E2E-REMOTO=falla", "E2E-REMOTO")
    term = next((l for l in s.screen("Shell").splitlines() if "E2E-TERM=" in l and "echo" not in l), "")
    check("%s: en el Shell, un servidor por ssh entendería el TERM (⌃L, clear, nano)" % w.label,
          line and line.strip().endswith("ok"), "%s · %s" % (term.strip(), (line or "").strip()))

    # Pegar: el texto del portapapeles llega al Shell (acción del atajo ⌘V).
    marker = "E2E-PEGAR-%d" % w.n
    board.set("echo %s-ok" % marker)
    key(w.terminal, "digit5")
    action(w.terminal, "paste_from_clipboard", "esc:v")
    key(w.terminal, "enter", mods=None)
    check("%s: ⌘V pega en el Shell (acción del atajo)" % w.label,
          wait_until(lambda: any(l.strip() == marker + "-ok" for l in s.screen("Shell").splitlines())))

    # Copiar en el editor: ⌘L selecciona la línea y ⌘C la copia (acción del atajo).
    key(w.terminal, "digit2")
    action(w.terminal, "esc:)")
    action(w.terminal, "copy_to_clipboard:mixed", "esc:c")
    check("%s: ⌘C en el editor deja la línea en el portapapeles de macOS (acción del atajo)" % w.label,
          wait_until(lambda: board.has("type HeaderProps"), timeout=4))
    key(w.terminal, "arrowLeft", mods=None)  # sin selección

    # Ratón: calibrar con dos clics en el Shell.
    if not calibrate(w, probe, tmp):
        check("%s: calibración del ratón (dos clics en el Shell)" % w.label, False, "sin eventos del ratón")
        return
    cw, ch, _, _ = w.cell
    # Seleccionar con el ratón en el Shell, con el foco en el editor: el mismo
    # arrastre enfoca el Shell y selecciona; Zellij lo copia al soltar. La
    # celda donde se suelta no entra en la selección: se suelta justo después.
    marker = "E2E-SELECCION-%d" % w.n
    shell_run(w, "clear; echo %s" % marker, marker)
    shell = s.pane("Shell")
    rows = s.screen("Shell").splitlines()
    y = next((i for i, l in enumerate(rows) if l.strip() == marker), None)
    if y is not None:
        x = rows[y].index(marker)
        start = point(w, shell["pane_content_x"] + x, shell["pane_content_y"] + y)
        end = point(w, shell["pane_content_x"] + x + len(marker), shell["pane_content_y"] + y)
        key(w.terminal, "digit2")
        board.set("e2e-vacio")
        drag(w.terminal, start, end)
    check("%s: arrastrar en el Shell sin el foco lo enfoca, selecciona y copia al soltar" % w.label,
          y is not None and wait_until(lambda: board.has(marker), timeout=4),
          "celda %.1fx%.1f pt · foco: %s" % (cw, ch, s.focused()))
    action(w.terminal, "copy_to_clipboard:mixed", "esc:c")
    check("%s: y ⌘C después no vacía el portapapeles (acción del atajo)" % w.label, board.has(marker))

    # Clic derecho en el explorador, sobre «src».
    key(w.terminal, "digit2")
    explorer = s.pane("Explorador")
    tree = s.explorer()
    y = next((i for i, l in enumerate(tree) if l.strip().endswith("src")), None)
    if y is not None:
        x = tree[y].index("src")
        mouse(w.terminal, *point(w, explorer["pane_content_x"] + x + 1, explorer["pane_content_y"] + y),
              button="right")
    opened = wait_until(lambda: (p := s.pane(MENU_TITLE)) and p["is_floating"], timeout=3)
    check("%s: el clic derecho en el explorador abre su menú" % w.label, opened,
          "foco: %s" % s.focused(), s)
    if opened:
        key(w.terminal, "escape", mods=None)
        wait_until(lambda: s.pane(MENU_TITLE) is None, timeout=3)

    # Clic derecho en el editor con el foco en el Shell, como en VS Code.
    key(w.terminal, "digit5")
    editor = s.pane("Editor")
    mouse(w.terminal, *point(w, editor["pane_content_x"] + 8, editor["pane_content_y"] + 2), button="right")
    opened = wait_until(lambda: (p := s.pane(MENU_TITLE)) and p["is_floating"], timeout=3)
    check("%s: el clic derecho en el editor sin el foco lo enfoca y abre el menú" % w.label, opened,
          "foco: %s" % s.focused(), s)
    if opened:
        key(w.terminal, "escape", mods=None)
        wait_until(lambda: s.pane(MENU_TITLE) is None, timeout=3)
    # Con el foco ya en el editor (lo que cubre scripts/smoke-test.py).
    key(w.terminal, "digit2")
    mouse(w.terminal, *point(w, editor["pane_content_x"] + 8, editor["pane_content_y"] + 2), button="right")
    opened = wait_until(lambda: (p := s.pane(MENU_TITLE)) and p["is_floating"], timeout=3)
    check("%s: el clic derecho en el editor con el foco abre el menú" % w.label, opened, session=s)
    if opened:
        key(w.terminal, "escape", mods=None)
        wait_until(lambda: s.pane(MENU_TITLE) is None, timeout=3)

    # ⌘K ⌘S: la ventana de atajos se abre en esta ventana (acción del atajo).
    action(w.terminal, "esc:?")
    shown = wait_until(lambda: (p := s.pane(SHORTCUTS_TITLE)) and p["is_floating"], timeout=4)
    check("%s: ⌘K ⌘S abre la ventana de atajos (acción del atajo)" % w.label, shown, session=s)
    if shown:
        action(w.terminal, "esc:?")
        wait_until(lambda: s.pane(SHORTCUTS_TITLE) is None, timeout=3)


def terminal_checks(w, board, tmp):
    print("\n%s" % w.label.capitalize())
    focus(w.terminal)
    log = tmp / "terminal-mode.txt"
    smoke.type_in(w.terminal, "clear; { whence -w ssh; echo TERM=$TERM; } > %s; echo E2E-TERMINAL-%d" % (log, w.n))
    wait_until(lambda: log.exists() and "TERM=" in log.read_text(), timeout=8)
    text = log.read_text() if log.exists() else ""
    check("%s: `ssh` es la función de Ghostty que lleva su terminfo al servidor" % w.label,
          "ssh: function" in text, text.replace("\n", " · ").strip())
    # Seleccionar con el ratón (selección de Ghostty) y copiar (acción del atajo ⌘C).
    action(w.terminal, "select_all")
    board.set("e2e-vacio")
    action(w.terminal, "copy_to_clipboard:mixed")
    check("%s: ⌘A ⌘C copia el texto de la pantalla (acción del atajo)" % w.label,
          wait_until(lambda: board.has("E2E-TERMINAL-%d" % w.n), timeout=4))


def main():
    if os.environ.get("ZELLIJ"):
        sys.exit("e2e-ghostty: ejecútalo fuera de Zellij (env -u ZELLIJ -u ZELLIJ_PANE_ID -u ZELLIJ_SESSION_NAME).")
    count = int(sys.argv[sys.argv.index("--windows") + 1]) if "--windows" in sys.argv else 4
    tmp = Path(tempfile.mkdtemp(prefix="vkterm-e2e-"))
    probe = tmp / "mouse-probe.py"
    probe.write_text(MOUSE_PROBE)
    print("vkTERM Test.app con %d ventanas (AppleScript de Ghostty; no son pulsaciones físicas)" % count)
    build = subprocess.run([str(ROOT / "bin" / "vkterm-app"), "install", "--test"], capture_output=True, text=True)
    app = Path(run(str(ROOT / "bin" / "vkterm-app"), "path", "--test").strip())
    if build.returncode != 0 or not app.is_dir():
        sys.exit("e2e-ghostty: no se pudo crear vkTERM Test.app: %s" % (build.stdout + build.stderr)[-300:])
    binary = app / "Contents" / "MacOS" / "ghostty"
    if run("pgrep", "-f", str(binary)).split():
        sys.exit("e2e-ghostty: vkTERM Test.app está abierta; ciérrala (⌘Q) y repite.")
    board_bin = tmp / "pasteboard"
    subprocess.run(["swiftc", "-O", "-o", str(board_bin), str(ROOT / "scripts" / "pasteboard.swift")],
                   check=True, capture_output=True)
    board = Pasteboard(board_bin)
    state = tmp / "state"
    before = smoke.sessions()
    subprocess.run(["open", "-a", str(app), "--env", "VKTERM_STATE_DIR=%s" % state,
                    "--env", "DISABLE_AUTO_UPDATE=" + smoke.OMZ_NO_UPDATE], check=True)
    pid = wait_until(lambda: int(run("pgrep", "-f", str(binary)).split()[0]), timeout=15)
    windows = []
    try:
        # La primera vez macOS pregunta (disco del repositorio, AppleScript):
        # hay que contestar en la pantalla.
        print("  (si macOS pregunta por vkTERM Test, acepta: la prueba espera hasta 3 minutos)", flush=True)
        ready = wait_until(lambda: smoke.app_terminal("count of windows") not in ("",), timeout=180, interval=2)
        if not ready:
            check("vkTERM Test.app responde a AppleScript", False, "¿diálogo de permisos sin contestar?")
            return
        terminals = []
        if smoke.app_terminal("count of windows") in ("", "0"):
            terminals.append(new_window())
        else:
            terminals.append(smoke.app_terminal("id of focused terminal of selected tab of front window"))
        while len(terminals) < count:
            terminals.append(new_window())
            time.sleep(1.0)
        check("se abren %d ventanas" % count, smoke.app_terminal("count of windows") == str(count),
              smoke.app_terminal("count of windows"))
        names = ["alfa", "beta", "gamma", "delta", "epsilon", "zeta"]
        for n, terminal in enumerate(terminals, 1):
            if n == count:
                windows.append(Window(n, terminal))
                continue
            project = tmp / names[n - 1]
            shutil.copytree(ROOT / "examples" / "demo", project)
            known = smoke.sessions()
            smoke.type_in(terminal, "cd %s && vkterm ." % project)
            name = wait_until(lambda: next((x for x in smoke.sessions() - known
                                            if x.startswith("vk-%s-" % names[n - 1])), None), timeout=40)
            s = Session(name) if name else None
            ok = s and wait_until(lambda: set(TITLES) <= {p["title"] for p in s.panes()}, timeout=25)
            if ok:
                smoke.grant_plugin(s)
            check("ventana %d: `vkterm .` la convierte en el IDE de %s" % (n, names[n - 1]), ok, name or "")
            windows.append(Window(n, terminal, project, s if ok else None))
        ides = [w for w in windows if w.s]

        for w in windows:
            if w.s:
                ide_checks(w, windows, board, tmp, probe)
            else:
                terminal_checks(w, board, tmp)

        print("\nCambiar de ventana")
        for round_ in (1, 2):
            for w in ides:
                focus(w.terminal)
                check("vuelta %d, %s: ⌘2 / ⌘1 llegan a su IDE" % (round_, w.label), probe_focus(w),
                      "foco: %s" % w.s.focused())

        print("\nRecargas de la configuración de Ghostty")
        first = ides[0]
        focus(first.terminal)
        action(first.terminal, "esc:t")  # ⌘K ⌘T
        picker = wait_until(lambda: (p := first.s.pane(smoke.THEMES_TITLE)) and p["is_focused"], timeout=5)
        if picker:
            key(first.terminal, "arrowDown", mods=None)
            key(first.terminal, "enter", mods=None)
            time.sleep(4.0)
        for w in ides:
            check("tras cambiar el tema en la ventana 1, %s sigue en el modo IDE" % w.label,
                  picker and (focus(w.terminal) or probe_focus(w)), "selector abierto=%s" % bool(picker))
        # Lo que pasa con «Reload Configuration» del menú, con un cambio entre
        # claro y oscuro de macOS o con cualquier recarga que no hace vkTERM.
        action(windows[-1].terminal, "reload_config")
        time.sleep(1.0)
        lost = [w.label for w in ides if not (focus(w.terminal) or probe_focus(w))]
        time.sleep(5.0)
        back = [w.label for w in ides if not (focus(w.terminal) or probe_focus(w))]
        check("tras una recarga que no hace vkTERM, el modo IDE se recupera solo en unos segundos", not back,
              "sin atajos justo después: %s; 6 s después: %s" % (lost or "ninguna", back or "ninguna"))

        print("\nCerrar una carpeta")
        last = ides[-1]
        focus(last.terminal)
        action(last.terminal, "esc:q")  # ⌘K F
        closed = wait_until(lambda: last.s.name not in smoke.sessions(), timeout=10)
        check("%s: ⌘K F cierra la carpeta y vuelve al modo terminal" % last.label, closed)
        for w in ides[:-1]:
            check("y %s sigue en el modo IDE" % w.label, focus(w.terminal) or probe_focus(w))
    finally:
        # Cada paso por separado: si uno falla (un `zellij list-sessions` que
        # no contesta), los demás se hacen igual y la app no se queda abierta.
        def cleanup(step):
            try:
                step()
            except (subprocess.SubprocessError, OSError) as error:
                print("  limpieza: %s" % error)

        cleanup(lambda: print("\nPortapapeles del usuario: %s" % {
            "restored": "devuelto", "kept": "no se toca (alguien copió algo)"}.get(board.restore(), "?")))
        cleanup(lambda: [run("zellij", "kill-session", created) for created in smoke.sessions() - before])
        if pid:
            time.sleep(1.0)
            cleanup(lambda: os.kill(pid, signal.SIGTERM))
            if not wait_until(lambda: not smoke.pid_alive(pid), timeout=10):
                cleanup(lambda: os.kill(pid, signal.SIGKILL))
        time.sleep(4.5)
        cleanup(lambda: [os.kill(int(leftover), signal.SIGTERM)
                         for leftover in run("pgrep", "-f", str(binary)).split()])
        shutil.rmtree(tmp, ignore_errors=True)


def summary():
    failed = [r for r in smoke.results if not r[1]]
    print("\n%d/%d comprobaciones correctas" % (len(smoke.results) - len(failed), len(smoke.results)))
    return 1 if failed or not smoke.results else 0


if __name__ == "__main__":
    try:
        main()
    finally:
        code = summary()
    sys.exit(code)
