#!/usr/bin/env python3
"""vkTERM — pruebas de humo (Fase 1: interfaz tipo VS Code).

    scripts/smoke-test.py             configuración + workspace en un PTY
    scripts/smoke-test.py --ghostty   además, Ghostty real con teclas sintetizadas

Modo PTY: arranca bin/vkterm sobre una copia temporal de examples/demo en un
pseudo-terminal de 200x50 y le envía exactamente los bytes que el perfil de
Ghostty emite para cada atajo ⌘ (leídos de `ghostty +list-keybinds`). El
estado se comprueba con `zellij action`, procesos y archivos en disco.

Modo --ghostty: abre vkTERM en una ventana de Ghostty (sin robarle el foco a
la app activa) y entrega eventos de teclado CGEvent al proceso de Ghostty con
scripts/send-keys.swift. Recorren Ghostty → Zellij → aplicación, pero NO son
pulsaciones físicas: la lista manual está en docs/manual-tests.md. Requiere
permiso de Accesibilidad para el terminal que ejecuta la prueba.

Nunca ejecuta Claude Code ni Codex, no modifica el portapapeles y no toca las
sesiones de Zellij que ya existían. Si Zellij pide permisos para el plugin
vkterm (solo la primera vez), la prueba los concede. Solo usa la biblioteca
estándar de Python 3 (incluido en las Xcode Command Line Tools).
"""
import fcntl
import json
import os
import pty
import re
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COLS, ROWS = 200, 50
GHOSTTY_APP = Path(os.environ.get("VKTERM_GHOSTTY_APP", "/Applications/Ghostty.app"))
GHOSTTY = GHOSTTY_APP / "Contents" / "MacOS" / "ghostty"
PROFILE = ROOT / "config" / "ghostty" / "vkterm.ghostty"
DEMO_FILE = "src/components/Header.tsx"
TITLES = ("Explorador", "Editor", "Shell", "Claude Code", "Codex")
TERMINALS = ("Editor", "Shell", "Claude Code", "Codex")
SHORTCUTS_TITLE = "Atajos de teclado"
THEMES_TITLE = "Tema de color"
# Caché de permisos de Zellij (solo se lee) y permisos que pide el plugin.
PERMISSIONS_CACHE = Path.home() / "Library" / "Caches" / "org.Zellij-Contributors.Zellij" / "permissions.kdl"
PLUGIN_PERMISSIONS = {"ReadApplicationState", "ChangeApplicationState", "WriteToStdin", "RunCommands",
                      "Reconfigure", "MessageAndLaunchOtherPlugins"}
# Temas de plugins/vkterm/src/themes.rs (id, nombre y colores del atenuado).
THEMES_RS = (ROOT / "plugins" / "vkterm" / "src" / "themes.rs").read_text()
THEMES = [dict(zip(("id", "name", "inactive_fg", "inactive_bg"), found)) for found in re.findall(
    r'id: "([^"]+)",\s*name: "([^"]+)",.*?inactive_fg: "([^"]+)",\s*inactive_bg: "([^"]+)"', THEMES_RS, re.S)]
DEFAULT_THEME = next(t for t in THEMES
                     if t["id"] == re.search(r'pub const DEFAULT: &str = "([^"]+)"', THEMES_RS).group(1))
# Fondo de los paneles sin foco con el tema predeterminado.
INACTIVE_BG = DEFAULT_THEME["inactive_bg"]

ENTER, SPACE, ESC = b"\r", b" ", b"\x1b"
UP, DOWN, RIGHT, LEFT, HOME = b"\x1b[A", b"\x1b[B", b"\x1b[C", b"\x1b[D", b"\x1b[H"
# Cabecera de un PNG: tiene bytes NUL, así que es binario.
PNG = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00"

# Atajo de macOS → acción esperada en el perfil de Ghostty.
EXPECTED_KEYBINDS = {
    "super+digit_1": "esc:1", "super+digit_2": "esc:2", "super+digit_3": "esc:3",
    "super+digit_4": "esc:4", "super+digit_5": "esc:5",
    "super+1": "esc:1", "super+2": "esc:2", "super+3": "esc:3",
    "super+4": "esc:4", "super+5": "esc:5",
    "super+b": "esc:6", "super+j": "esc:7", "super+alt+b": "esc:8", "super+alt+key_b": "esc:8",
    "super+w": "esc:9", "super+p": "esc:0", "super+k>super+s": "esc:?", "super+k>z": "esc:z",
    "super+k>super+t": "esc:t",
    "super+alt+arrow_left": "esc:,", "super+alt+arrow_right": "esc:.",
    "ctrl+shift+tab": "esc:,", "ctrl+tab": "esc:.",
    "super+alt+arrow_up": "esc:k", "super+alt+arrow_down": "esc:j",
    "super+s": "esc:OQ", "super+arrow_left": "csi:H", "super+arrow_right": "csi:F",
    "super+c": "copy_to_clipboard:mixed|esc:c", "super+v": "paste_from_clipboard",
    "super+z": "esc:u", "super+shift+z": "esc:y", "super+a": "esc:a", "super+f": "esc:s",
    "super+x": "esc:x",
}
UNBOUND = ("super+t", "super+d", "super+shift+d")
SHORTCUTS = {"⌘1": "super+digit_1", "⌘2": "super+digit_2", "⌘3": "super+digit_3",
             "⌘4": "super+digit_4", "⌘5": "super+digit_5", "⌘B": "super+b", "⌘J": "super+j",
             "⌥⌘B": "super+alt+b", "⌘W": "super+w", "⌘P": "super+p", "⌘S": "super+s",
             "⌘⌥←": "super+alt+arrow_left", "⌘⌥→": "super+alt+arrow_right",
             "⌃Tab": "ctrl+tab", "⌃⇧Tab": "ctrl+shift+tab",
             "⌘⌥↑": "super+alt+arrow_up", "⌘⌥↓": "super+alt+arrow_down",
             "⌘←": "super+arrow_left", "⌘→": "super+arrow_right",
             "⌘K ⌘S": "super+k>super+s", "⌘K Z": "super+k>z", "⌘K ⌘T": "super+k>super+t",
             "⌘Z": "super+z", "⌘⇧Z": "super+shift+z", "⌘A": "super+a", "⌘F": "super+f",
             "⌘X": "super+x", "⌘C": "super+c"}


def alt(key):
    return b"\x1b" + key.encode()


def ctrl(key):
    return bytes([ord(key) & 0x1F])


def paste(text):
    """Lo que Ghostty envía con ⌘V a una aplicación con bracketed paste."""
    return b"\x1b[200~" + text.encode() + b"\x1b[201~"


class Client:
    """A Zellij client running in its own pseudo-terminal."""

    def __init__(self, argv, cwd):
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(cwd)
            os.environ["TERM"] = "xterm-256color"
            # Tema y estado de Micro propios: no tocan los del usuario.
            os.environ["VKTERM_STATE_DIR"] = str(Path(cwd) / "state")
            os.execv(argv[0], argv)
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
        os.kill(self.pid, signal.SIGWINCH)
        self.recorded = None
        threading.Thread(target=self._drain, daemon=True).start()

    def _drain(self):
        try:
            while chunk := os.read(self.fd, 65536):
                if self.recorded is not None:
                    self.recorded += chunk
        except OSError:
            pass

    def record(self):
        """Empieza a guardar lo que Zellij dibuja en el terminal."""
        self.recorded = bytearray()

    def send(self, *keys, pause=0.35):
        for key in keys:
            os.write(self.fd, key)
            time.sleep(pause)

    def alive(self):
        return os.waitpid(self.pid, os.WNOHANG) == (0, 0)


class GhosttyKeys:
    """Teclas sintetizadas entregadas al proceso de Ghostty (scripts/send-keys.swift)."""

    def __init__(self, binary, pid):
        self.binary, self.pid = binary, pid

    def send(self, *combos, pause=0.35):
        subprocess.run([str(self.binary), str(self.pid), *combos], check=True, timeout=30)
        time.sleep(pause)


def wait_until(fn, timeout=6.0, interval=0.2):
    end = time.time() + timeout
    while time.time() < end:
        try:
            value = fn()
        except (subprocess.SubprocessError, OSError, ValueError, KeyError, IndexError, TypeError):
            value = None
        if value:
            return value
        time.sleep(interval)
    return None


def run(*argv, **kwargs):
    return subprocess.run(argv, capture_output=True, text=True, timeout=30, **kwargs).stdout


def sessions():
    return set(run("zellij", "list-sessions", "--short", "--no-formatting").split())


class Session:
    def __init__(self, name):
        self.name = name

    def action(self, *args):
        return run("zellij", "-s", self.name, "action", *args)

    def panes(self):
        # Se descartan solo los plugins de servicio (barra y zellij:link).
        # Incluye los paneles ocultos (is_suppressed).
        return [p for p in json.loads(self.action("list-panes", "-a", "-j"))
                if p["plugin_url"] not in ("zellij:link", "compact-bar")
                and p["title"] != "compact-bar"]

    def pane(self, title):
        return next((p for p in self.panes() if p["title"] == title), None)

    def focused(self):
        return next((p["title"] for p in self.panes() if p["is_focused"] and not p["is_suppressed"]), None)

    def geometry(self):
        """Posición y tamaño de los paneles visibles."""
        return {p["title"]: (p["pane_x"], p["pane_y"], p["pane_columns"], p["pane_rows"])
                for p in self.panes() if not p["is_suppressed"]}

    def hidden(self):
        return {p["title"] for p in self.panes() if p["is_suppressed"]}

    def screen(self, title_or_pane):
        pane = self.pane(title_or_pane) if isinstance(title_or_pane, str) else title_or_pane
        return self.action("dump-screen", "-p", pane_id(pane))

    def plugin_view(self, plugin_id, ansi=False):
        """Viewport de un plugin (dump-screen no cubre plugins)."""
        proc = subprocess.Popen(
            ["zellij", "-s", self.name, "subscribe", "-p", plugin_id, "-f", "json"]
            + (["--ansi"] if ansi else []),
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        time.sleep(1.0)
        proc.terminate()
        out, _ = proc.communicate(timeout=5)
        frames = [json.loads(line) for line in out.splitlines() if line.startswith("{")]
        return "\n".join(frames[-1]["viewport"]) if frames else ""

    def explorer(self):
        """Filas que dibuja el explorador (sin espacios a la derecha)."""
        return [l.rstrip() for l in self.plugin_view(pane_id(self.pane("Explorador"))).splitlines()]

    def dimmed(self):
        """Terminales con el fondo de los paneles sin foco."""
        return {p["title"] for p in self.panes()
                if not p["is_plugin"] and (p.get("default_bg") or "").lower() == INACTIVE_BG}

    def mode(self):
        bar = next(p for p in json.loads(self.action("list-panes", "-a", "-j"))
                   if p["title"] == "compact-bar")
        view = self.plugin_view(pane_id(bar))
        return next((m for m in ("LOCKED", "NORMAL", "PANE") if m in view), "?")


def pane_id(pane):
    return ("plugin_%d" if pane["is_plugin"] else "terminal_%d") % pane["id"]


def micro_status(s):
    """Barra de estado de Micro en el panel Editor ("archivo (línea,col) | ft:...").
    Se toma la primera: si el panel se encogió, dump-screen añade filas antiguas debajo."""
    return next((l for l in s.screen("Editor").splitlines()
                 if re.search(r"\(\d+,\d+\) \| ft:", l)), "")


def micro_tabs(s):
    """Barra de pestañas de Micro (solo existe con más de una); la activa va entre []."""
    first = (s.screen("Editor").splitlines() or [""])[0]
    return first if "[" in first and not re.match(r"\s*\d+(\s|$)", first) else ""


def micro_prompt(s):
    return next((l for l in s.screen("Editor").splitlines() if l.startswith("> ")), "")


def micro_processes(s):
    return [c for c in server_children(s.name) if os.path.basename(c["command"].split()[0]) == "micro"]


def server_children(session):
    server = run("pgrep", "-f", "zellij --server .*%s$" % session).split()
    if not server:
        return []
    children = []
    for row in run("ps", "-A", "-o", "pid=,ppid=,tty=,command=").splitlines():
        pid, ppid, tty, command = row.split(None, 3)
        if ppid == server[0]:
            children.append({"pid": int(pid), "tty": tty, "command": command})
    return children


def pid_alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def ghostty_cli(*args):
    """Ghostty CLI con solo el perfil de vkTERM cargado (sin la config del usuario)."""
    with tempfile.TemporaryDirectory() as xdg:
        (Path(xdg) / "ghostty").mkdir()
        (Path(xdg) / "ghostty" / "config").write_text("config-file = %s\n" % PROFILE)
        return run(str(GHOSTTY), *args, env=dict(os.environ, XDG_CONFIG_HOME=xdg))


def ghostty_keybinds():
    """Atajo → acción; las acciones encadenadas (`chain=`) se unen con "|"."""
    binds, last = {}, None
    for line in ghostty_cli("+list-keybinds", "--plain").splitlines():
        if line.startswith("keybind = "):
            trigger, _, action = line[len("keybind = "):].partition("=")
            if trigger == "chain" and last:
                binds[last] += "|" + action
            else:
                binds[trigger], last = action, trigger
    return binds


def action_bytes(action):
    """Bytes que llegan a Zellij; de una cadena, los de su parte esc:/csi:."""
    if "|" in action:
        action = next(a for a in action.split("|") if a.startswith(("esc:", "csi:")))
    if action.startswith("esc:"):
        return b"\x1b" + action[4:].encode()
    if action.startswith("csi:"):
        return b"\x1b[" + action[4:].encode()
    raise ValueError("acción sin bytes: %s" % action)


results = []


def check(name, ok, detail="", session=None):
    results.append((name, bool(ok), detail))
    print("  %s %s%s" % ("✔" if ok else "✘", name, "  — " + detail if detail else ""), flush=True)
    if not ok and session:
        for p in session.panes():
            print("      %s %-28s focused=%s hidden=%s" % (
                pane_id(p), p["title"][-28:], p["is_focused"], p["is_suppressed"]))


def new_project(tmp, scenario):
    project = tmp / scenario / "demo"
    shutil.copytree(ROOT / "examples" / "demo", project)
    return project


def wait_new_session(before, prefix="vk-demo-", timeout=15):
    return wait_until(lambda: next(iter(sorted(s for s in sessions() - before
                                               if s.startswith(prefix))), None), timeout)


def grant_plugin(s):
    """La primera vez, Zellij pide permisos para el plugin vkterm en su panel."""
    explorer = wait_until(lambda: s.pane("Explorador"), timeout=20)
    if explorer and "Allow?" in s.plugin_view(pane_id(explorer)):
        s.action("send-keys", "-p", pane_id(explorer), "y")
        wait_until(lambda: "Allow?" not in s.plugin_view(pane_id(explorer)), timeout=10)


# ---------------------------------------------------------------- configuración

def check_config():
    print("Configuración")
    cmd = {name: action_bytes(EXPECTED_KEYBINDS[trigger]) for name, trigger in SHORTCUTS.items()}
    build = subprocess.run([str(ROOT / "scripts" / "build-plugins.sh"), "--test"],
                           capture_output=True, text=True, timeout=900)
    summary = next((l for l in build.stdout.splitlines() if l.startswith("test result")), "")
    check("plugin vkterm compilado y tests de su lógica (cargo test)", build.returncode == 0,
          summary or build.stderr.strip()[-120:])
    zcheck = run("zellij", "--config-dir", str(ROOT / "config" / "zellij"), "setup", "--check",
                 env=dict(os.environ, VKTERM_HOME=str(ROOT)))
    check("zellij setup --check acepta config.kdl", "Well defined" in zcheck)
    if not GHOSTTY.exists():
        check("Ghostty instalado", False, "se omiten las comprobaciones del perfil")
        return cmd
    valid = subprocess.run([str(GHOSTTY), "+validate-config", "--config-file=%s" % PROFILE],
                           capture_output=True, text=True)
    check("ghostty +validate-config acepta el perfil", valid.returncode == 0, valid.stdout.strip())
    effective = ghostty_cli("+show-config")
    check("perfil efectivo: los dos Option escriben caracteres (como VS Code)",
          "macos-option-as-alt = false" in effective)
    # El tema puede venir de config/local/theme.ghostty (⌘K ⌘T).
    check("perfil efectivo: título y tema de vkTERM",
          "title = vkTERM" in effective and re.search(r"^theme = \S", effective, re.M))
    launcher = (ROOT / "bin" / "vkterm-ghostty").read_text()
    check("el lanzador no carga ~/.config/ghostty", "--config-default-files=false" in launcher)
    binds = ghostty_keybinds()
    wrong = {t: binds.get(t) for t, a in EXPECTED_KEYBINDS.items() if binds.get(t) != a}
    check("atajos ⌘ efectivos de Ghostty", not wrong, "distintos: %s" % wrong if wrong else
          "%d atajos traducidos" % len(EXPECTED_KEYBINDS))
    check("splits y pestañas de Ghostty desactivados", not any(t in binds for t in UNBOUND))
    check("⌘C nunca envía Ctrl-C", binds.get("super+c", "").startswith("copy_to_clipboard"))
    check("⌘W ya no cierra la ventana", binds.get("super+w") == "esc:9")
    # Los bytes que la prueba envía salen de la configuración efectiva.
    return {name: action_bytes(binds[trigger]) for name, trigger in SHORTCUTS.items()
            if trigger in binds
            and any(part[:4] in ("esc:", "csi:") for part in binds[trigger].split("|"))} or cmd


# --------------------------------------------------------------------- workspace

def near(value, expected, tolerance=2):
    return abs(value - expected) <= tolerance


def check_workspace(s):
    """Comprueba el layout de arranque y devuelve su geometría."""
    ready = wait_until(lambda: set(TITLES) <= {p["title"] for p in s.panes()}, timeout=20)
    check("arranca con explorador, editor, Shell, Claude Code y Codex", ready, session=s)
    if not ready:
        return None
    grant_plugin(s)
    check("el explorador es el plugin vkterm", s.pane("Explorador")["plugin_url"] == "vkterm")
    check("sin paneles apilados ni ocultos", "stacked" not in s.action("dump-layout")
          and not s.hidden())
    g = s.geometry()
    ex, ed, sh, cl, co = (g[t] for t in TITLES)
    width = ex[2] + ed[2] + cl[2]
    layout_ok = (ex[0] == 0 and ex[1] == 0 and ed[0] == sh[0] == ex[0] + ex[2]
                 and ed[2] == sh[2] and sh[1] > ed[1] and cl[0] == co[0] == ed[0] + ed[2]
                 and co[1] > cl[1] and near(ex[3], ed[3] + sh[3]))
    check("layout tipo VS Code: explorador | editor sobre Shell | Claude Code sobre Codex",
          layout_ok, str(g))
    check("proporciones 20 / 55 / 25 y editor 70 / Shell 30",
          near(ex[2], width * 0.20) and near(cl[2], width * 0.25)
          and near(sh[3], (ed[3] + sh[3]) * 0.30),
          "columnas %s, filas %s" % ([ex[2], ed[2], cl[2]], [ed[3], sh[3]]))
    waiting = all("Waiting to run" in s.screen(t) for t in ("Claude Code", "Codex"))
    agents = [c for c in server_children(s.name)
              if os.path.basename(c["command"].split()[0]) in ("claude", "codex")]
    check("Claude Code y Codex preparados, sin ejecutarse", waiting and not agents)
    status = wait_until(lambda: (lambda l: DEMO_FILE in l and l)(micro_status(s)))
    check("el editor abre %s" % DEMO_FILE, status and "No name" not in status, (status or "")[:60])
    tree = wait_until(lambda: (lambda v: "▸ src" in "\n".join(v) and v)(s.explorer()))
    check("el explorador muestra el proyecto como árbol", tree and tree[0].strip() == "DEMO",
          " | ".join(l.strip() for l in (tree or [])[:3]))
    check("modo inicial LOCKED", s.mode() == "LOCKED")
    return s.geometry()


# ---------------------------------------------------------------- escenario PTY

def scenario_pty(cmd, tmp):
    project = new_project(tmp, "pty")
    launcher = [str(ROOT / "bin" / "vkterm"), str(project)]
    before = sessions()
    client = Client(launcher, tmp)
    name = wait_new_session(before, timeout=60)
    if not name:
        check("la sesión de Zellij arranca", False)
        return
    s = Session(name)
    print("\nSesión %s sobre %s" % (name, project))
    try:
        print("Workspace")
        if check_workspace(s):
            keyboard_pty(s, client, cmd)
            editor_pty(s, client, cmd, project)
            explorer_pty(s, client, cmd, project)
            toggles_pty(s, client, cmd)
            dimming_pty(s, client, cmd)
            shortcuts_pty(s, client, cmd)
            themes_pty(s, client, cmd, launcher, tmp)
            shell_pty(s, client, cmd, project)
            client, sleeper = processes_pty(s, client, tmp)
            sessions_pty(s, client, launcher, tmp, sleeper)
    finally:
        for created in sessions() - before:
            run("zellij", "kill-session", created)


def click(client, pane, line):
    """Clic izquierdo (protocolo SGR) en la fila `line` del contenido de un panel."""
    x, y = pane["pane_content_x"] + 4, pane["pane_content_y"] + line + 1
    client.send(b"\x1b[<0;%d;%dM" % (x, y), pause=0.05)
    client.send(b"\x1b[<0;%d;%dm" % (x, y), pause=0.8)


def focus_matrix(s, press, steps):
    for label, expected in steps:
        press(label)
        got = wait_until(lambda: s.focused() == expected and expected, timeout=3)
        check("%s → %s" % (label, expected), got, "" if got else "foco en %s" % s.focused())


def keyboard_pty(s, client, cmd):
    print("⌘1–⌘5 por identidad y ⌘⌥ ↑ ↓ (bytes que emite Ghostty, modo locked)")
    focus_matrix(s, lambda k: client.send(cmd[k]), (
        ("⌘5", "Shell"), ("⌘3", "Claude Code"), ("⌘4", "Codex"), ("⌘1", "Explorador"),
        ("⌘2", "Editor"), ("⌘⌥↓", "Shell"), ("⌘⌥↑", "Editor"), ("⌘4", "Codex"),
        ("⌘5", "Shell"), ("⌘1", "Explorador"), ("⌘3", "Claude Code"), ("⌘2", "Editor")))
    print("Alt (otros terminales con Option como Alt)")
    focus_matrix(s, lambda k: client.send(alt(k[-1])), (
        ("Alt h", "Explorador"), ("Alt l", "Editor"), ("Alt j", "Shell"), ("Alt k", "Editor")))
    client.send(cmd["⌘K Z"])
    zoomed = wait_until(lambda: s.pane("Editor")["is_fullscreen"], timeout=3)
    client.send(cmd["⌘K Z"])
    restored = wait_until(lambda: not s.pane("Editor")["is_fullscreen"], timeout=3)
    check("⌘K Z amplía y restaura el panel", zoomed and restored)
    client.send(cmd["⌘5"], cmd["⌘P"], pause=0.8)
    prompt = wait_until(lambda: s.focused() == "Editor" and micro_prompt(s).startswith("> tab"), timeout=3)
    check("⌘P va al editor y prepara «tab» en la línea de comandos de Micro", prompt, session=s)
    client.send(ESC)


def editor_pty(s, client, cmd, project):
    print("Editor: guardar, pegar, abrir")
    target = project / DEMO_FILE
    client.send(cmd["⌘2"], b"// vkterm-cmd-s", ENTER, cmd["⌘S"])
    check("⌘S guarda en Micro", wait_until(lambda: target.read_text().startswith("// vkterm-cmd-s\n")))
    check("⌘S no cambia el modo de Zellij", s.mode() == "LOCKED")
    client.send(b"// vkterm-ctrl-s", ENTER, ctrl("s"))
    check("Ctrl-S (Micro) sigue guardando", wait_until(lambda: "// vkterm-ctrl-s" in target.read_text()))
    client.send(paste("// vkterm-paste\r"), cmd["⌘S"])
    check("⌘V (bracketed paste) inserta texto en Micro",
          wait_until(lambda: "// vkterm-paste\n" in target.read_text()))
    client.send(cmd["⌘P"], b"src/notes.ts", ENTER, pause=0.8)
    check("⌘P abre un archivo en una pestaña nueva del editor",
          wait_until(lambda: micro_status(s).startswith("src/notes.ts ")
                     and "[src/notes.ts]" in micro_tabs(s) and DEMO_FILE in micro_tabs(s)),
          micro_tabs(s).strip())
    notes = project / "src" / "notes.ts"
    client.send(b"vk-middle", cmd["⌘←"], b"//H", cmd["⌘→"], b"//E", cmd["⌘S"])
    check("⌘← / ⌘→ van al inicio / fin de línea en Micro",
          wait_until(lambda: notes.read_text().splitlines()[0] == "//Hvk-middle//E"))
    client.send(b" vk-undo", pause=0.6)
    typed = wait_until(lambda: "vk-undo" in s.screen("Editor"))
    client.send(cmd["⌘Z"], pause=0.8)
    undone = wait_until(lambda: "vk-undo" not in s.screen("Editor"))
    client.send(cmd["⌘⇧Z"], pause=0.8)
    check("⌘Z / ⌘⇧Z deshacen y rehacen en Micro",
          typed and undone and wait_until(lambda: "vk-undo" in s.screen("Editor")))
    client.send(cmd["⌘F"], pause=0.8)
    found = wait_until(lambda: any(l.startswith("Find") for l in s.screen("Editor").splitlines()))
    client.send(ESC, pause=0.5)
    check("⌘F abre la búsqueda de Micro", found)
    client.send(cmd["⌘A"], b"vk-all", cmd["⌘S"], pause=0.8)
    check("⌘A selecciona todo (lo escrito lo sustituye)",
          wait_until(lambda: notes.read_text().strip() == "vk-all"), notes.read_text()[:30])
    client.send(cmd["⌘W"], pause=0.8)
    check("⌘W cierra la pestaña activa del editor",
          wait_until(lambda: micro_status(s).startswith(DEMO_FILE + " ") and not micro_tabs(s)),
          micro_status(s)[:50])


def explorer_pty(s, client, cmd, project):
    print("Explorador (árbol) → pestañas del editor")
    (project / "src" / "logo.png").write_bytes(PNG)
    panes = len(s.panes())
    client.send(cmd["⌘1"], RIGHT, pause=0.6)
    check("→ despliega src/", wait_until(lambda: (lambda v: "▾ src" in v and "▸ components" in v)(
        "\n".join(s.explorer()))))
    client.send(RIGHT, RIGHT, pause=0.6)
    check("→ baja a components/ y la despliega",
          wait_until(lambda: (lambda v: "▾ components" in v and "Header.tsx" in v)("\n".join(s.explorer()))))
    client.send(RIGHT, DOWN, ENTER, pause=1.0)
    opened = wait_until(lambda: s.focused() == "Editor" and micro_status(s).startswith("src/App.tsx ")
                        and "[src/App.tsx]" in micro_tabs(s) and DEMO_FILE in micro_tabs(s))
    check("Enter en App.tsx lo abre como pestaña del editor y le da el foco", opened,
          micro_tabs(s).strip(), s)
    check("abrir archivos no crea paneles ni otro Micro",
          len(s.panes()) == panes and len(micro_processes(s)) == 1, session=s)
    client.send(cmd["⌘1"], UP, SPACE, pause=1.0)
    tabs = micro_tabs(s)
    check("Espacio abre sin quitar el foco al explorador; un archivo ya abierto salta a su pestaña",
          wait_until(lambda: micro_status(s).startswith(DEMO_FILE + " ")) and s.focused() == "Explorador"
          and micro_tabs(s).count(DEMO_FILE) == 1 and micro_tabs(s).count("src/App.tsx") == 1,
          micro_tabs(s).strip(), s)
    client.send(DOWN, DOWN, ENTER, pause=1.0)
    footer = wait_until(lambda: (lambda v: "binario" in v[-1] and v[-1])(s.explorer()))
    check("un archivo binario no se abre: aviso en el explorador",
          footer and micro_tabs(s) == tabs and s.focused() == "Explorador", (footer or "").strip())
    client.send(HOME, ENTER, pause=0.6)
    collapsed = wait_until(lambda: "▸ src" in "\n".join(s.explorer()))
    client.send(ENTER, pause=0.6)
    check("Enter en una carpeta la pliega y despliega, sin abrir terminales",
          collapsed and wait_until(lambda: "▾ src" in "\n".join(s.explorer()))
          and len(s.panes()) == panes)
    client.send(LEFT, LEFT, LEFT, pause=0.6)
    tree = s.explorer()
    check("← en la raíz no sale del proyecto", tree[0].strip() == "DEMO" and tree[1].strip() == "▸ src",
          " | ".join(l.strip() for l in tree[:3]))
    client.send(b".", SPACE, pause=1.0)
    check("una letra salta a la entrada que empieza por ella",
          wait_until(lambda: micro_status(s).startswith(".vkterm ")), micro_status(s)[:40])
    explorer = s.pane("Explorador")
    click(client, explorer, s.explorer().index(" ▸ src"))
    expanded = wait_until(lambda: "▾ src" in "\n".join(s.explorer()))
    row = next((i for i, l in enumerate(s.explorer()) if l.strip() == "App.tsx"), None)
    if row is not None:
        click(client, explorer, row)
    check("clic: despliega la carpeta y abre el archivo",
          expanded and wait_until(lambda: micro_status(s).startswith("src/App.tsx ")),
          micro_status(s)[:40])
    client.send(cmd["⌘1"], b".", SPACE, pause=1.0)
    print("Pestañas del editor")
    client.send(cmd["⌘⌥←"], pause=0.6)
    check("⌘⌥← pestaña anterior (y foco en el editor)",
          wait_until(lambda: micro_status(s).startswith("src/App.tsx ") and s.focused() == "Editor"),
          micro_status(s)[:40])
    client.send(cmd["⌘⌥→"], pause=0.6)
    check("⌘⌥→ pestaña siguiente", wait_until(lambda: micro_status(s).startswith(".vkterm ")))
    client.send(cmd["⌃Tab"], pause=0.6)
    check("⌃Tab siguiente (vuelve a la primera)",
          wait_until(lambda: micro_status(s).startswith(DEMO_FILE + " ")))
    client.send(cmd["⌃⇧Tab"], pause=0.6)
    check("⌃⇧Tab anterior", wait_until(lambda: micro_status(s).startswith(".vkterm ")))
    client.send(cmd["⌘5"], cmd["⌘W"], pause=0.8)
    check("⌘W fuera del editor no cierra nada",
          micro_status(s).startswith(".vkterm ") and s.focused() == "Shell" and len(s.panes()) == panes)
    client.send(cmd["⌘2"], cmd["⌘W"], pause=0.8)
    check("⌘W en el editor cierra la pestaña",
          wait_until(lambda: ".vkterm" not in micro_tabs(s) and micro_tabs(s)), micro_tabs(s).strip())


def toggles_pty(s, client, cmd):
    print("⌘B / ⌘J / ⌥⌘B: mostrar y ocultar sin cerrar procesos")
    g0 = s.geometry()
    pids = sorted(c["pid"] for c in server_children(s.name))
    client.send(cmd["⌘B"], pause=0.8)
    g = s.geometry()
    check("⌘B oculta el explorador y el editor ocupa su espacio",
          s.hidden() == {"Explorador"} and g["Editor"][0] == 0
          and g["Editor"][2] == g0["Editor"][2] + g0["Explorador"][2], str(g.get("Editor")), s)
    client.send(cmd["⌘B"], pause=0.8)
    check("⌘B lo muestra en el mismo sitio y tamaño", wait_until(lambda: s.geometry() == g0), session=s)
    client.send(cmd["⌘5"], cmd["⌘J"], pause=0.8)
    g = s.geometry()
    check("⌘J oculta el Shell, el editor ocupa la columna y recibe el foco",
          s.hidden() == {"Shell"} and g["Editor"][3] == g0["Editor"][3] + g0["Shell"][3]
          and s.focused() == "Editor", "", s)
    client.send(cmd["⌘J"], pause=0.8)
    check("⌘J lo muestra en su sitio y le da el foco (como VS Code)",
          wait_until(lambda: s.geometry() == g0) and s.focused() == "Shell", session=s)
    client.send(cmd["⌥⌘B"], pause=0.8)
    g = s.geometry()
    check("⌥⌘B oculta Claude Code y Codex",
          s.hidden() == {"Claude Code", "Codex"}
          and g["Editor"][2] == g0["Editor"][2] + g0["Claude Code"][2], "", s)
    client.send(cmd["⌘3"], pause=1.0)
    check("⌘3 con los agentes ocultos los muestra y enfoca Claude Code",
          wait_until(lambda: s.geometry() == g0) and s.focused() == "Claude Code", session=s)
    client.send(cmd["⌘B"], cmd["⌥⌘B"], cmd["⌘J"], pause=0.8)
    g = s.geometry()
    check("con todo oculto, el editor ocupa la ventana",
          set(g) == {"Editor"} and g["Editor"][:3] == (0, 0, COLS), str(g))
    client.send(cmd["⌥⌘B"], cmd["⌘B"], cmd["⌘J"], pause=0.8)
    check("mostrando en otro orden se recupera el layout exacto",
          wait_until(lambda: s.geometry() == g0), str(s.geometry()), s)
    client.send(cmd["⌘J"], cmd["⌘B"], pause=0.8)
    client.send(cmd["⌘5"], pause=1.0)
    client.send(cmd["⌘1"], pause=1.0)
    check("⌘5 y ⌘1 muestran el Shell y el explorador ocultos",
          wait_until(lambda: s.geometry() == g0) and s.focused() == "Explorador", session=s)
    check("ningún proceso cambia al ocultar y mostrar",
          sorted(c["pid"] for c in server_children(s.name)) == pids)
    s.action("write", "-p", pane_id(s.pane("Codex")), "27")
    wait_until(lambda: "Waiting to run" not in s.screen("Codex"), timeout=3)
    client.send(cmd["⌥⌘B"], pause=0.8)
    client.send(cmd["⌥⌘B"], pause=0.8)
    check("Codex con un shell en vez del agente vuelve también a su sitio",
          wait_until(lambda: s.geometry() == g0), str(s.geometry()), s)


def background_sgr(color):
    """SGR de 24 bits del fondo "#rrggbb", como lo devuelve `zellij subscribe --ansi`."""
    return "48;2;%d;%d;%d" % tuple(int(color[i:i + 2], 16) for i in (1, 3, 5))


def foreground_colors(view):
    return set(re.findall(r"38;2;\d+;\d+;\d+", view))


def dimming_pty(s, client, cmd):
    print("Paneles sin foco atenuados (como iTerm2)")
    for key, focused in (("⌘2", "Editor"), ("⌘5", "Shell"), ("⌘3", "Claude Code"),
                         ("⌘4", "Codex"), ("⌘1", "Explorador")):
        client.send(cmd[key], pause=0.8)
        expected = set(TERMINALS) - {focused}
        check("%s: %s con sus colores, el resto atenuado" % (key, focused),
              wait_until(lambda: s.dimmed() == expected), str(sorted(s.dimmed())), s)
    sgr = background_sgr(INACTIVE_BG)
    explorer = pane_id(s.pane("Explorador"))
    view = s.plugin_view(explorer, ansi=True)
    check("el explorador con el foco se dibuja con los colores normales",
          "DEMO" in re.sub(r"\x1b\[[0-9;]*m", "", view) and sgr not in view)
    client.send(cmd["⌘2"], pause=0.8)
    check("el explorador sin foco usa el mismo fondo que los terminales",
          wait_until(lambda: sgr in s.plugin_view(explorer, ansi=True), timeout=4))
    # Zellij reinicia los colores de un panel en espera al redimensionarlo.
    client.send(cmd["⌘3"], pause=0.8)
    client.record()
    client.send(cmd["⌘B"], pause=0.8)
    client.send(cmd["⌘B"], pause=0.8)
    check("tras ⌘B, los paneles redimensionados siguen atenuados",
          wait_until(lambda: s.dimmed() == set(TERMINALS) - {"Claude Code"}),
          str(sorted(s.dimmed())), s)
    check("al volver, el marco del explorador no muestra SCROLL",
          b"SCROLL:" not in client.recorded)
    client.recorded = None


def cached_permissions():
    wasm = ROOT / "plugins" / "vkterm" / "target" / "wasm32-wasip1" / "release" / "vkterm.wasm"
    found = re.search(r'"/%s" \{([^}]*)\}' % re.escape(str(wasm)), PERMISSIONS_CACHE.read_text())
    return set(found.group(1).split()) if found else set()


def shortcuts_pty(s, client, cmd):
    print("⌘K ⌘S: ventana de atajos (como Unity)")
    client.send(cmd["⌘2"], pause=0.8)
    client.send(cmd["⌘K ⌘S"], pause=1.0)
    window = wait_until(lambda: (p := s.pane(SHORTCUTS_TITLE)) and p["is_floating"] and p["is_focused"] and p)
    check("se abre como ventana flotante con el foco", window, session=s)
    if not window:
        return
    window = wait_until(lambda: (p := s.pane(SHORTCUTS_TITLE)) and p["pane_columns"] > 80 and p) or window
    x, y, w, h = window["pane_x"], window["pane_y"], window["pane_columns"], window["pane_rows"]
    check("centrada y del tamaño de su contenido",
          near(x, (COLS - w) // 2) and near(y, (ROWS - 1 - h) // 2) and w < COLS and h < ROWS,
          "x=%d y=%d %dx%d" % (x, y, w, h))
    view = s.plugin_view(pane_id(window))
    rows = view.splitlines()

    def column(heading):
        """Columna en la que empieza el título de una categoría."""
        pattern = r"(^|\s{2})(%s)(\s{2}|$)" % re.escape(heading)
        return next((m.start(2) for r in rows if (m := re.search(pattern, r))), None)
    left = {column(h) for h in ("Paneles", "Editor", "Explorador")}
    right = {column(h) for h in ("Agentes y Shell", "Zellij", "Teclado")}
    check("categorías en columnas de tres, como Unity",
          len(left) == 1 and len(right) == 1 and None not in left | right and max(left) < min(right),
          "columnas %s y %s" % (sorted(left, key=str), sorted(right, key=str)))
    missing = [name for name in SHORTCUTS if not all(part in view for part in name.split())]
    check("incluye todos los atajos ⌘ del perfil de Ghostty", not missing, ", ".join(missing))
    check("el resto de paneles queda atenuado", wait_until(lambda: s.dimmed() == set(TERMINALS)),
          str(sorted(s.dimmed())))
    client.send(cmd["⌘K ⌘S"], pause=1.0)
    check("⌘K ⌘S otra vez la cierra y el foco vuelve al editor",
          wait_until(lambda: s.pane(SHORTCUTS_TITLE) is None and s.focused() == "Editor"), session=s)
    client.send(cmd["⌘K ⌘S"], pause=1.0)
    opened = wait_until(lambda: (p := s.pane(SHORTCUTS_TITLE)) and p["is_focused"])
    client.send(ESC, pause=1.0)
    check("Esc la cierra", opened and wait_until(lambda: s.pane(SHORTCUTS_TITLE) is None), session=s)
    client.send(cmd["⌘K ⌘S"], pause=1.0)
    opened = wait_until(lambda: (p := s.pane(SHORTCUTS_TITLE)) and p["is_focused"])
    client.send(cmd["⌘5"], pause=1.0)
    check("se cierra al pasar el foco a otro panel (⌘5)",
          opened and wait_until(lambda: s.pane(SHORTCUTS_TITLE) is None and s.focused() == "Shell"),
          session=s)
    check("no deja paneles de más", len(s.panes()) == len(TITLES), session=s)
    # Zellij guarda los últimos permisos pedidos por el plugin: si la ventana
    # pidiera menos, el explorador volvería a preguntar en la siguiente sesión.
    check("los permisos del explorador siguen guardados", PLUGIN_PERMISSIONS <= cached_permissions(),
          ", ".join(sorted(cached_permissions())))


def themes_pty(s, client, cmd, launcher, tmp):
    print("⌘K ⌘T: tema de color (como VS Code)")
    state = tmp / "state" / "theme"

    def saved():
        found = state.exists() and re.search(r"^id=(.*)$", state.read_text(), re.M)
        return found.group(1) if found else ""

    def picker_opened():
        client.send(cmd["⌘K ⌘T"], pause=1.0)
        return wait_until(lambda: (p := s.pane(THEMES_TITLE)) and p["is_floating"] and p["is_focused"] and p)

    def explorer_dimmed_with(theme):
        return wait_until(lambda: background_sgr(theme["inactive_bg"])
                          in s.plugin_view(explorer, ansi=True), timeout=5)

    def editor_reversed():
        """Barras de Micro en vídeo inverso (SGR 7): esquema de los temas claros."""
        return re.search(r"\x1b\[(?:[0-9;]*;)?7(?:;[0-9;]*)?m",
                         s.plugin_view(pane_id(s.pane("Editor")), ansi=True))

    explorer = pane_id(s.pane("Explorador"))
    bar = pane_id(next(p for p in json.loads(s.action("list-panes", "-a", "-j")) if p["title"] == "compact-bar"))
    default_bar = foreground_colors(s.plugin_view(bar, ansi=True))
    dark_editor = not editor_reversed()
    second = THEMES[1]
    client.send(cmd["⌘2"], pause=0.8)
    picker = picker_opened()
    check("se abre el selector flotante con el foco", picker, session=s)
    if not picker:
        return
    view = wait_until(lambda: (v := s.plugin_view(pane_id(picker))) and "●" in v and v) or ""
    check("lista todos los temas y marca el guardado",
          all(t["name"] in view for t in THEMES)
          and any("●" in r and DEFAULT_THEME["name"] in r for r in view.splitlines()))
    client.send(DOWN, pause=1.5)
    check("↓ prueba el siguiente tema y lo guarda", wait_until(lambda: saved() == second["id"]), saved())
    check("el atenuado cambia al del tema", explorer_dimmed_with(second))
    check("Zellij cambia de tema en la sesión",
          wait_until(lambda: foreground_colors(s.plugin_view(bar, ansi=True)) - default_bar, timeout=5))
    client.send(ESC, pause=1.5)
    check("Esc cierra y vuelve al tema que había",
          wait_until(lambda: s.pane(THEMES_TITLE) is None and saved() == DEFAULT_THEME["id"])
          and explorer_dimmed_with(DEFAULT_THEME)
          and wait_until(lambda: foreground_colors(s.plugin_view(bar, ansi=True)) == default_bar, timeout=5),
          saved(), s)
    if picker_opened():
        client.send(DOWN, pause=1.0)
        client.send(ENTER, pause=1.5)
    check("Enter deja el tema elegido",
          wait_until(lambda: s.pane(THEMES_TITLE) is None and saved() == second["id"])
          and explorer_dimmed_with(second), saved(), s)
    check("Micro cambia a su esquema de tema claro", dark_editor and wait_until(editor_reversed, timeout=6))
    second_bar = foreground_colors(s.plugin_view(bar, ansi=True))
    before = sessions()
    Client([launcher[0], "--new", launcher[1]], tmp)
    fresh = wait_new_session(before, timeout=30)
    if fresh:
        other = Session(fresh)
        grant_plugin(other)
        other_bar = pane_id(next(p for p in json.loads(other.action("list-panes", "-a", "-j"))
                                 if p["title"] == "compact-bar"))
        check("una sesión nueva arranca con el tema guardado",
              wait_until(lambda: foreground_colors(other.plugin_view(other_bar, ansi=True)) == second_bar)
              and wait_until(lambda: background_sgr(second["inactive_bg"])
                             in other.plugin_view(pane_id(other.pane("Explorador")), ansi=True), timeout=8))
        run("zellij", "kill-session", fresh)
    else:
        check("una sesión nueva arranca con el tema guardado", False, "no arrancó")
    if picker_opened():
        client.send(HOME, pause=1.0)
        client.send(ENTER, pause=1.5)
    check("se puede volver al predeterminado",
          wait_until(lambda: s.pane(THEMES_TITLE) is None and saved() == DEFAULT_THEME["id"])
          and explorer_dimmed_with(DEFAULT_THEME), saved(), s)


def shell_pty(s, client, cmd, project):
    print("Shell: sin conflictos con ⌘S y ⌘V")
    shell = s.pane("Shell")
    client.send(cmd["⌘5"])
    wait_until(lambda: s.screen(shell).strip())
    client.send(cmd["⌘S"], b"echo vk-f2-ok", ENTER, pause=0.8)
    screen = wait_until(lambda: (lambda t: "\nvk-f2-ok" in t and t)(s.screen(shell)))
    check("⌘S en el prompt no deja basura", screen and "OQ" not in screen)
    client.send(b"cho vk-home", cmd["⌘←"], b"e", cmd["⌘→"], b"-end", ENTER, pause=0.8)
    check("⌘← / ⌘→ van al inicio / fin de línea en el shell",
          wait_until(lambda: "\nvk-home-end" in s.screen(shell)))
    s.action("write-chars", "-p", pane_id(shell), "for i in 1 2 3 4 5 6 7 8; do echo vkline$i; sleep 0.4; done")
    s.action("send-keys", "-p", pane_id(shell), "Enter")
    time.sleep(1.0)
    count = lambda: s.screen(shell).count("vkline")
    client.send(cmd["⌘S"])
    first = count()
    time.sleep(1.6)
    check("⌘S no congela la salida de un comando (no es Ctrl-S)", count() > first,
          "líneas %d → %d" % (first, count()))
    time.sleep(2.5)
    client.send(paste("echo vk-no-exec\r"), pause=0.8)
    lines = s.screen(shell).splitlines()
    check("⌘V en el shell no ejecuta lo pegado", not any(l.strip() == "vk-no-exec" for l in lines))
    client.send(ctrl("c"), pause=0.8)
    # ⌘Z, ⌘C y ⌘X solo actúan en el editor: aquí serían Ctrl-Z, Ctrl-C y Ctrl-X.
    s.action("write-chars", "-p", pane_id(shell), "sleep 30")
    s.action("send-keys", "-p", pane_id(shell), "Enter")
    def sleep_in_panes():
        panes = {str(c["pid"]) for c in server_children(s.name)}
        rows = (r.split(None, 2) for r in run("ps", "-A", "-o", "pid=,ppid=,command=").splitlines())
        return next(int(pid) for pid, ppid, command in rows if ppid in panes and command == "sleep 30")
    sleeper = wait_until(sleep_in_panes)
    client.send(cmd["⌘Z"], cmd["⌘C"], cmd["⌘X"], pause=0.8)
    stat = run("ps", "-o", "stat=", "-p", str(sleeper)).strip() if sleeper else ""
    check("⌘Z, ⌘C y ⌘X en el shell no suspenden ni interrumpen el proceso",
          sleeper and pid_alive(sleeper) and not stat.startswith("T"), stat)
    client.send(ctrl("c"), pause=0.8)
    s.action("write-chars", "-p", pane_id(shell), "echo $EDITOR > .vk-editor; zellij edit src/App.tsx")
    s.action("send-keys", "-p", pane_id(shell), "Enter")
    check("`zellij edit` desde el shell abre en el editor central",
          wait_until(lambda: s.focused() == "Editor" and micro_status(s).startswith("src/App.tsx ")
                     and micro_tabs(s).count("src/App.tsx") == 1 and len(s.panes()) == len(TITLES)),
          session=s)
    check("$EDITOR sigue siendo micro (git, agentes)",
          wait_until(lambda: (project / ".vk-editor").read_text() == "micro\n"))


def processes_pty(s, client, tmp):
    print("Paneles y procesos")
    client.send(alt("5"), ctrl("q"))
    check("Ctrl-q no cierra la sesión", s.name in sessions() and s.mode() == "LOCKED")
    # Dos pestañas abiertas (Header.tsx y App.tsx): Ctrl-q en cada una cierra Micro.
    client.send(alt("2"), ctrl("q"), pause=0.8)
    client.send(ctrl("q"), pause=2.0)
    restarted = wait_until(lambda: micro_status(s).startswith("No name") and not s.pane("Editor")["exited"])
    check("al cerrar Micro (última pestaña), el editor vuelve a abrirse vacío", restarted,
          micro_status(s)[:40], s)
    existing = {pane_id(p) for p in s.panes()}
    client.send(ctrl("g"))
    normal = s.mode() == "NORMAL"
    client.send(b"p")
    pane_mode = s.mode() == "PANE"
    client.send(b"n", pause=1.0)
    new = wait_until(lambda: next((p for p in s.panes() if pane_id(p) not in existing), None))
    check("Ctrl-g p n crea un panel y vuelve a LOCKED",
          normal and pane_mode and new and s.mode() == "LOCKED")
    pids = []
    for pane, label in ((s.pane("Shell"), "shell"), (new, "new")):
        pidfile = tmp / ("%s.pid" % label)
        command = "echo $$ > %s%s" % (pidfile, "; sleep 300" if label == "new" else "")
        # Un zsh recién abierto puede descartar lo que se escribe mientras arranca.
        for _ in range(3):
            s.action("write-chars", "-p", pane_id(pane), command)
            s.action("send-keys", "-p", pane_id(pane), "Enter")
            if wait_until(lambda: pidfile.read_text().strip(), timeout=3):
                pids.append(int(pidfile.read_text()))
                break
    ttys = [run("ps", "-o", "tty=", "-p", str(p)).strip() for p in pids]
    check("cada panel tiene su propio proceso y PTY",
          len(set(pids)) == 2 and len(set(ttys)) == 2, "%s %s" % (pids, ttys), s)
    if len(pids) < 2:
        return client, None
    sleeper = wait_until(lambda: int(run("pgrep", "-P", str(pids[1]), "sleep").split()[0]))
    micro = next(c["pid"] for c in server_children(s.name) if c["command"].startswith("micro"))
    os.kill(pids[0], signal.SIGKILL)
    held = wait_until(lambda: s.pane("Shell")["exited"])
    check("matar el shell de un panel no afecta a los demás (el Shell espera Enter)",
          held and sleeper and pid_alive(sleeper) and pid_alive(micro)
          and s.pane("Claude Code")["is_held"])
    return client, sleeper


def sessions_pty(s, client, launcher, tmp, sleeper):
    print("Sesiones")
    client.send(ctrl("g"), b"o", b"d")
    detached = wait_until(lambda: not client.alive())
    check("Ctrl-g o d desconecta y la sesión sigue viva",
          detached and s.name in sessions() and (sleeper is None or pid_alive(sleeper)))
    client = Client(launcher, tmp)
    clients = wait_until(lambda: len(s.action("list-clients").splitlines()) == 2)
    check("el mismo comando reanuda la misma sesión", clients and (sleeper is None or pid_alive(sleeper)))
    before = sessions()
    other = Client([launcher[0], "--new", launcher[1]], tmp)
    fresh = wait_new_session(before)
    check("--new crea otra sesión sin tocar la existente",
          fresh and fresh.startswith(s.name + "-") and s.name in sessions(), fresh or "")
    if fresh:
        run("zellij", "kill-session", fresh)
    run("zellij", "kill-session", s.name)
    wait_until(lambda: s.name not in sessions())
    before = sessions()
    client = Client(launcher, tmp)
    restarted = wait_new_session(before)
    check("tras cerrar la sesión, el mismo comando la recrea con el mismo nombre",
          restarted == s.name, restarted or "")
    if restarted:
        print("  Tras el reinicio:")
        check_workspace(s)


# ------------------------------------------------------------- escenario Ghostty

def scenario_ghostty(tmp):
    print("\nGhostty (teclas sintetizadas con CGEvent; no son pulsaciones físicas)")
    if not GHOSTTY.exists() or not shutil.which("swiftc"):
        check("Ghostty y swiftc disponibles", False)
        return
    keys_bin = tmp / "send-keys"
    subprocess.run(["swiftc", "-O", "-o", str(keys_bin), str(ROOT / "scripts" / "send-keys.swift")],
                   check=True, capture_output=True)
    project = new_project(tmp, "ghostty")
    before, ghostty_before = sessions(), set(run("pgrep", "-x", "ghostty").split())
    subprocess.run([str(ROOT / "bin" / "vkterm-ghostty"), "--new", str(project)], check=True,
                   env=dict(os.environ, VKTERM_STATE_DIR=str(tmp / "ghostty-state")))
    pid = wait_until(lambda: int(next(iter(set(run("pgrep", "-x", "ghostty").split())
                                           - ghostty_before))), timeout=10)
    name = wait_new_session(before, timeout=30)
    if not (pid and name):
        check("Ghostty abre vkTERM", False,
              "¿falta el permiso de macOS para que Ghostty lea el disco del repositorio?")
        if pid:
            os.kill(pid, signal.SIGTERM)
        return
    s = Session(name)
    try:
        args = run("ps", "-o", "command=", "-p", str(pid))
        check("Ghostty arranca con el perfil de vkTERM y sin la config global",
              "--config-default-files=false" in args and str(PROFILE) in args)
        check("cliente de Zellij conectado desde Ghostty",
              wait_until(lambda: len(s.action("list-clients").splitlines()) == 2, timeout=10))
        if not check_workspace(s):
            return
        keyboard_ghostty(s, GhosttyKeys(keys_bin, pid), project, pid)
    finally:
        run("zellij", "kill-session", name)
        if not wait_until(lambda: not pid_alive(pid), timeout=10):
            os.kill(pid, signal.SIGTERM)


def keyboard_ghostty(s, keys, project, pid):
    print("Atajos en Ghostty")
    focus_matrix(s, lambda k: keys.send(k), (
        ("cmd+5", "Shell"), ("cmd+3", "Claude Code"), ("cmd+4", "Codex"), ("cmd+1", "Explorador"),
        ("cmd+2", "Editor"), ("cmd+lopt+down", "Shell"), ("cmd+ropt+up", "Editor"),
        ("cmd+5", "Shell")))
    keys.send("cmd+2", "cmd+k", "z")
    zoomed = wait_until(lambda: s.pane("Editor")["is_fullscreen"], timeout=3)
    keys.send("cmd+k", "z")
    check("⌘K Z amplía y restaura el panel",
          zoomed and wait_until(lambda: not s.pane("Editor")["is_fullscreen"], timeout=3))
    keys.send("cmd+5", "ctrl+c", "text:echo", "space", "lopt+2", "ropt+2", "ropt+3", pause=0.8)
    line = next((l for l in reversed(s.screen("Shell").splitlines()) if "echo " in l), "")
    typed = line.split("echo ", 1)[-1].split()[:1]
    check("los dos Option escriben caracteres (⌥2 = @, ⌥3 = #) y no navegan",
          s.focused() == "Shell" and typed == ["@@#"], "escribió %s" % typed)
    keys.send("ctrl+c")

    target = project / DEMO_FILE
    keys.send("cmd+2", "text:vkcmds", "enter", "cmd+s", pause=1.0)
    check("⌘S guarda en Micro", wait_until(lambda: target.read_text().startswith("vkcmds\n")))
    keys.send("text:vkctrls", "enter", "ctrl+s", pause=1.0)
    check("Ctrl-S guarda en Micro", wait_until(lambda: "vkctrls\n" in target.read_text()))
    keys.send("cmd+right", "text:e", "cmd+left", "text:h", "cmd+s", pause=1.0)
    check("⌘← / ⌘→ en Micro", wait_until(lambda: target.read_text().splitlines()[2]
                                           == "htype HeaderProps = {e"))
    keys.send("cmd+backspace", "cmd+s", pause=1.0)
    check("⌘⌫ no activa la grabación de macros en Micro",
          target.read_text().splitlines()[2] == "htype HeaderProps = {e"
          and "Recording" not in s.screen("Editor"))
    keys.send("cmd+5", "cmd+p", pause=0.8)
    check("⌘P va al editor y prepara «tab» en Micro",
          wait_until(lambda: s.focused() == "Editor" and micro_prompt(s).startswith("> tab"), timeout=3))
    keys.send("esc")

    clipboard = run("osascript", "-e", "clipboard info")
    text = run("pbpaste")
    lines = [l.strip() for l in text.splitlines() if len(l.strip()) >= 6]
    sample = lines[-1][:20] if lines else ""
    if sample:
        # Se pega en una pestaña vacía que se cierra sin guardar; la comparación
        # es en memoria y el contenido no se muestra. ⌘← lleva la vista al
        # principio de la última línea pegada.
        keys.send("cmd+p", "enter", pause=0.8)
        keys.send("cmd+v", pause=1.2)
        keys.send("cmd+left", pause=0.6)
        check("⌘V pega el portapapeles en Micro",
              wait_until(lambda: micro_status(s).startswith("No name +") and sample in s.screen("Editor"),
                         timeout=4), "contenido no mostrado")
        keys.send("ctrl+q", pause=0.6)
        # Solo si Micro pregunta: si la pestaña no cambió, Ctrl-Q ya la cerró y
        # la "n" se escribiría en Header.tsx.
        if wait_until(lambda: "Save changes" in s.screen("Editor"), timeout=3):
            keys.send("text:n", pause=0.8)
        # "+" tras el nombre: Header.tsx modificado.
        closed = wait_until(lambda: micro_status(s).startswith(DEMO_FILE + " ")
                            and not micro_status(s).startswith(DEMO_FILE + " +") and not micro_tabs(s))
        line = next((l for l in s.screen("Editor").splitlines() if "type HeaderProps" in l), "")
        stray = line.split("type HeaderProps")[0][-1:] if "htype" not in line else ""
        check("la pestaña de prueba se cierra sin guardar", closed,
              "" if closed else "%s; carácter de más: %s" % (micro_status(s)[:30],
                                                            "n" if stray == "n" else "otro"))
    else:
        check("⌘V (omitido: el portapapeles no tiene texto)", True)
    keys.send("cmd+5", "text:sleep", "space", "3", "0", "enter", pause=0.8)
    sleeper = wait_until(lambda: [c for c in server_children(s.name)] and int(
        run("pgrep", "-n", "-x", "sleep").split()[0]))
    keys.send("cmd+c", pause=0.8)
    check("⌘C no interrumpe el proceso del shell ni toca el portapapeles",
          sleeper and pid_alive(sleeper) and run("osascript", "-e", "clipboard info") == clipboard)
    keys.send("ctrl+c")
    keys.send("ctrl+g", pause=0.6)
    normal = s.mode() == "NORMAL"
    keys.send("esc", pause=0.6)
    check("Ctrl-g desbloquea Zellij y Esc vuelve a LOCKED", normal and s.mode() == "LOCKED")

    keys.send("cmd+1", "right", "right", "right", "right", "down", "enter", pause=1.0)
    check("Explorador: → despliega y Enter abre App.tsx como pestaña del editor",
          wait_until(lambda: s.focused() == "Editor" and micro_status(s).startswith("src/App.tsx ")
                     and DEMO_FILE in micro_tabs(s) and len(s.panes()) == len(TITLES)),
          micro_tabs(s).strip(), s)
    keys.send("cmd+lopt+left", pause=0.6)
    previous = wait_until(lambda: micro_status(s).startswith(DEMO_FILE + " "), timeout=3)
    keys.send("cmd+ropt+right", pause=0.6)
    following = wait_until(lambda: micro_status(s).startswith("src/App.tsx "), timeout=3)
    keys.send("ctrl+tab", pause=0.6)
    tab = wait_until(lambda: micro_status(s).startswith(DEMO_FILE + " "), timeout=3)
    keys.send("ctrl+shift+tab", pause=0.6)
    check("⌘⌥← / ⌘⌥→ y ⌃Tab / ⌃⇧Tab cambian de pestaña",
          previous and following and tab
          and wait_until(lambda: micro_status(s).startswith("src/App.tsx "), timeout=3))
    keys.send("cmd+lopt+left", pause=0.6)
    wait_until(lambda: micro_status(s).startswith(DEMO_FILE + " "), timeout=3)

    g0 = s.geometry()
    keys.send("cmd+b", pause=0.8)
    hidden = s.hidden() == {"Explorador"}
    keys.send("cmd+b", pause=0.8)
    check("⌘B oculta y muestra el explorador en su sitio",
          hidden and wait_until(lambda: s.geometry() == g0), session=s)
    keys.send("cmd+j", pause=0.8)
    hidden = s.hidden() == {"Shell"}
    keys.send("cmd+j", pause=0.8)
    check("⌘J oculta y muestra el Shell (con el foco)",
          hidden and wait_until(lambda: s.geometry() == g0) and s.focused() == "Shell", session=s)
    keys.send("cmd+lopt+b", pause=0.8)
    hidden = s.hidden() == {"Claude Code", "Codex"}
    keys.send("cmd+ropt+b", pause=0.8)
    check("⌥⌘B (con cualquiera de los dos ⌥) oculta y muestra los agentes",
          hidden and wait_until(lambda: s.geometry() == g0), session=s)
    keys.send("cmd+w", pause=0.8)
    check("⌘W fuera del editor no cierra la ventana ni la pestaña",
          pid_alive(pid) and len(s.action("list-clients").splitlines()) == 2
          and DEMO_FILE in micro_tabs(s))
    keys.send("cmd+2", "cmd+w", pause=0.8)
    check("⌘W en el editor cierra la pestaña",
          wait_until(lambda: not micro_tabs(s) and micro_status(s).startswith("src/App.tsx "), timeout=3)
          and pid_alive(pid), micro_status(s)[:40])
    keys.send("cmd+k", "cmd+s", pause=1.0)
    opened = wait_until(lambda: (p := s.pane(SHORTCUTS_TITLE)) and p["is_floating"] and p["is_focused"])
    keys.send("esc", pause=1.0)
    check("⌘K ⌘S abre la ventana de atajos y Esc la cierra",
          opened and wait_until(lambda: s.pane(SHORTCUTS_TITLE) is None and s.focused() == "Editor"),
          session=s)
    keys.send("cmd+k", "cmd+t", pause=1.0)
    themes = wait_until(lambda: (p := s.pane(THEMES_TITLE)) and p["is_floating"] and p["is_focused"])
    keys.send("esc", pause=1.0)
    check("⌘K ⌘T abre el selector de tema y Esc lo cierra",
          themes and wait_until(lambda: s.pane(THEMES_TITLE) is None and s.focused() == "Editor"), session=s)

    def column():
        found = re.search(r"\((\d+),(\d+)\)", micro_status(s))
        return found and (int(found.group(1)), int(found.group(2)))
    keys.send("cmd+left", pause=0.6)
    start = column()
    keys.send("ropt+right", pause=0.6)
    word = column()
    keys.send("lopt+left", pause=0.6)
    check("⌥→ / ⌥← saltan palabra en el editor, con los dos ⌥",
          start and word and word[0] == start[0] and word[1] > start[1] + 1 and column() == start,
          "%s → %s → %s" % (start, word, column()))


def main():
    if os.environ.get("ZELLIJ"):
        sys.exit("smoke-test: ejecútalo fuera de Zellij.")
    for tool in ("zellij", "micro", "jq"):
        if not shutil.which(tool):
            sys.exit("smoke-test: falta %s (ver scripts/doctor.sh)." % tool)
    previous = sessions()
    tmp = Path(tempfile.mkdtemp(prefix="vkterm-smoke-"))
    try:
        cmd = check_config()
        scenario_pty(cmd, tmp)
        if "--ghostty" in sys.argv[1:]:
            scenario_ghostty(tmp)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    print("\nSesiones")
    check("las sesiones que ya existían siguen vivas", previous <= sessions(),
          ", ".join(sorted(previous)) or "no había ninguna")
    failed = [r for r in results if not r[1]]
    print("\n%d/%d comprobaciones correctas" % (len(results) - len(failed), len(results)))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
