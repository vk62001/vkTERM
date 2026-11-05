#!/usr/bin/env python3
"""Tests de bin/vkterm-theme (python3 -m unittest scripts/test_vkterm_theme.py).

Qué Ghostty se recarga al cambiar de tema o de fuente: solo el que dibuja el
terminal de la sesión. scripts/smoke-test.py abre el IDE en un pseudoterminal
propio; ejecutado desde una pestaña de vkTERM.app, subía por los procesos
padre hasta el Ghostty de esa pestaña y lo recargaba, y las pestañas del IDE
del usuario se quedaban sin atajos ⌘. El «Ghostty» de estas pruebas es un
programa en C llamado ghostty que ejecuta un comando en un pseudoterminal
nuevo (forkpty), como Ghostty con el shell de cada pestaña; necesita cc
(Xcode Command Line Tools)."""
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
THEME = ROOT / "bin" / "vkterm-theme"
GHOSTTY_C = r"""
#include <sys/wait.h>
#include <unistd.h>
#include <util.h>
int main(int argc, char **argv) {
    int fd;
    char buf[4096];
    pid_t pid = forkpty(&fd, NULL, NULL, NULL);
    if (pid == 0) {
        execvp(argv[1], argv + 1);
        _exit(127);
    }
    while (read(fd, buf, sizeof buf) > 0) {
    }
    waitpid(pid, NULL, 0);
    return 0;
}
"""
# Un programa en otro pseudoterminal: lo que hace scripts/smoke-test.py.
NESTED_PTY = """
import os, pty, sys, time
pid, fd = pty.fork()
if pid == 0:
    os.execv("/bin/sleep", ["sleep", "30"])
open(sys.argv[1], "w").write(str(pid))
time.sleep(30)
"""


def comm(pid):
    return subprocess.run(["ps", "-o", "comm=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()


class TerminalOf(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        source = Path(self.tmp.name) / "ghostty.c"
        source.write_text(GHOSTTY_C)
        self.ghostty = Path(self.tmp.name) / "ghostty"
        subprocess.run(["cc", "-o", str(self.ghostty), str(source), "-lutil"], check=True)
        self.procs = []

    def tearDown(self):
        for proc in self.procs:
            proc.kill()
            proc.wait()
        self.tmp.cleanup()

    def sleeper(self, *command):
        """Arranca el «Ghostty» con `command`, que escribe el pid de un
        `sleep` en el archivo que recibe al final; devuelve ese pid."""
        pidfile = Path(self.tmp.name) / ("pid%d" % len(self.procs))
        proc = subprocess.Popen([str(self.ghostty), *command, str(pidfile)], start_new_session=True,
                                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.procs.append(proc)
        for _ in range(50):
            if pidfile.exists() and pidfile.read_text().strip():
                return int(pidfile.read_text())
            time.sleep(0.1)
        self.fail("el «Ghostty» de la prueba no arrancó su terminal")

    def terminal(self, pid):
        return subprocess.run([str(THEME), "terminal", str(pid)], capture_output=True, text=True).stdout.strip()

    def test_el_ghostty_que_dibuja_el_terminal(self):
        # Ghostty → (pseudoterminal) → shell → programa, como una pestaña real.
        pid = self.sleeper("/bin/sh", "-c", 'sleep 30 & echo $! > "$0"; wait')
        found = self.terminal(pid)
        self.assertTrue(found and Path(comm(found)).name == "ghostty", "%r → %r" % (pid, found))

    def test_no_el_de_fuera_de_otro_pseudoterminal(self):
        # Ghostty → (pseudoterminal) → python → (otro pseudoterminal) → programa.
        pid = self.sleeper("python3", "-c", NESTED_PTY)
        self.assertEqual(self.terminal(pid), "")


if __name__ == "__main__":
    unittest.main()
