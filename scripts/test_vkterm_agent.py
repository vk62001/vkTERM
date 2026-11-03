#!/usr/bin/env python3
"""Tests de bin/vkterm-agent (python3 -m unittest scripts/test_vkterm_agent.py).

La traducción de Super+V aislada y el intermediario de verdad, en un
pseudoterminal: lo que recibe la orden, el tamaño, el código de salida, ⌃Z y
sin terminal."""
import fcntl
import importlib.machinery
import importlib.util
import os
import pty
import select
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
AGENT = Path(__file__).resolve().parent.parent / "bin" / "vkterm-agent"
loader = importlib.machinery.SourceFileLoader("vkterm_agent", str(AGENT))
spec = importlib.util.spec_from_loader("vkterm_agent", loader)
agent = importlib.util.module_from_spec(spec)
loader.exec_module(agent)

# La orden de prueba: anota lo que recibe (en modo raw) y el tamaño al
# arrancar y con cada SIGWINCH; con "q" termina con el código 3.
RECORDER = r'''
import fcntl, os, signal, struct, sys, termios, tty
log = open(sys.argv[1], "ab", buffering=0)
def size(*_):
    rows, cols = struct.unpack("HHHH", fcntl.ioctl(0, termios.TIOCGWINSZ, b"\0" * 8))[:2]
    log.write(b"<%d %d>" % (rows, cols))
signal.signal(signal.SIGWINCH, size)
size()
tty.setraw(0)
os.write(1, b"listo\r\n")
while True:
    try:
        data = os.read(0, 1024)
    except InterruptedError:
        continue
    log.write(data)
    if b"q" in data:
        sys.exit(3)
'''


class Translate(unittest.TestCase):
    def test_super_v_es_ctrl_v(self):
        self.assertEqual(agent.translate(b"\x1b[118;9u"), (b"\x16", b""))
        self.assertEqual(agent.translate(b"a\x1b[118;9:1ub"), (b"a\x16b", b""))
        # Con códigos alternativos (flag 4 del protocolo de kitty).
        self.assertEqual(agent.translate(b"\x1b[118::86;9u"), (b"\x16", b""))
        # Con el texto asociado (flag 16).
        self.assertEqual(agent.translate(b"\x1b[118;9;118u"), (b"\x16", b""))

    def test_soltar_y_repetir_no_envian_nada(self):
        self.assertEqual(agent.translate(b"\x1b[118;9:3u\x1b[118;9:2u"), (b"", b""))

    def test_lo_demas_pasa_tal_cual(self):
        for data in (b"\x1b[118;5u", b"\x1b[118;10u", b"\x1b[1;5A", b"\x1b[200~hola\x1b[201~", b"\x16"):
            self.assertEqual(agent.translate(data), (data, b""))

    def test_secuencia_partida_queda_pendiente(self):
        self.assertEqual(agent.translate(b"x\x1b[11"), (b"x", b"\x1b[11"))
        self.assertEqual(agent.translate(b"\x1b"), (b"", b"\x1b"))
        self.assertEqual(agent.translate(b"\x1b[118;9"), (b"", b"\x1b[118;9"))


class Proxy(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="vkterm-agent-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.out = b""

    def start(self, *argv):
        pid, fd = pty.fork()
        if pid == 0:
            os.execv(str(AGENT), [str(AGENT), *argv])
        self.addCleanup(os.close, fd)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
        return pid, fd

    def pump(self, fd, seconds):
        end = time.time() + seconds
        while time.time() < end:
            if select.select([fd], [], [], 0.05)[0]:
                try:
                    self.out += os.read(fd, 4096)
                except OSError:
                    return

    def test_la_orden_recibe_ctrl_v_y_lo_demas_igual(self):
        (self.tmp / "recorder.py").write_text(RECORDER)
        log = self.tmp / "log"
        pid, fd = self.start(sys.executable, "-I", str(self.tmp / "recorder.py"), str(log))
        self.pump(fd, 1.5)
        for keys in (b"abc", b"\x1b[118;9u", (b"\x1b[11", b"8;9u"), b"\x1b[118;5u",
                     b"\x1b[200~pegado\x1b[201~", b"\x1b"):
            if isinstance(keys, tuple):
                # Partida en dos lecturas seguidas: se junta.
                os.write(fd, keys[0])
                time.sleep(0.005)
                os.write(fd, keys[1])
            else:
                os.write(fd, keys)
            self.pump(fd, 0.15)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        os.kill(pid, signal.SIGWINCH)
        self.pump(fd, 0.4)
        os.write(fd, b"q")
        self.pump(fd, 1.0)
        _, code = os.waitpid(pid, 0)
        self.assertIn(b"listo", self.out)
        self.assertEqual(log.read_bytes(),
                         b"<30 100>abc\x16\x16\x1b[118;5u\x1b[200~pegado\x1b[201~\x1b<40 120>q")
        self.assertEqual(os.WEXITSTATUS(code), 3)

    def test_ctrl_z_detiene_y_fg_reanuda(self):
        pid, fd = self.start("sh", "-c", "echo antes; kill -TSTP $$; echo vuelto; exit 4")
        self.pump(fd, 1.0)
        done, code = os.waitpid(pid, os.WNOHANG | os.WUNTRACED)
        self.assertEqual(done, pid)
        self.assertTrue(os.WIFSTOPPED(code), "el intermediario se detiene con la orden")
        os.kill(pid, signal.SIGCONT)
        self.pump(fd, 1.0)
        _, code = os.waitpid(pid, 0)
        self.assertEqual(self.out.replace(b"\r", b"").split(b"\n")[:2], [b"antes", b"vuelto"])
        self.assertEqual(os.WEXITSTATUS(code), 4)

    def test_orden_inexistente(self):
        pid, fd = self.start("vkterm-agent-no-existe")
        self.pump(fd, 1.5)
        _, code = os.waitpid(pid, 0)
        self.assertIn(b"vkterm-agent-no-existe", self.out)
        self.assertEqual(os.WEXITSTATUS(code), 127)

    def test_sin_terminal_ejecuta_la_orden_tal_cual(self):
        done = subprocess.run([str(AGENT), "cat"], input=b"hola", capture_output=True)
        self.assertEqual((done.stdout, done.returncode), (b"hola", 0))
        done = subprocess.run([str(AGENT), "sh", "-c", "exit 5"], stdin=subprocess.DEVNULL)
        self.assertEqual(done.returncode, 5)
        done = subprocess.run([str(AGENT), "vkterm-agent-no-existe"], stdin=subprocess.DEVNULL,
                              capture_output=True)
        self.assertEqual(done.returncode, 127)


if __name__ == "__main__":
    unittest.main()
