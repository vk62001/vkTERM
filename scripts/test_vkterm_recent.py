#!/usr/bin/env python3
"""Tests de bin/vkterm-recent (python3 -m unittest scripts/test_vkterm_recent.py).

La lista de «Abrir reciente» del modo terminal: nombres, filtro, teclas
(clásicas y del protocolo de kitty) y, en un pseudoterminal, la orden de
verdad con un estado de vkTERM propio."""
import codecs
import importlib.machinery
import importlib.util
import os
import pty
import select
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent.parent
RECENT = ROOT / "bin" / "vkterm-recent"
loader = importlib.machinery.SourceFileLoader("vkterm_recent", str(RECENT))
spec = importlib.util.spec_from_loader("vkterm_recent", loader)
recent = importlib.util.module_from_spec(spec)
loader.exec_module(recent)


def keys(data):
    found, rest = recent.parse_keys(data, codecs.getincrementaldecoder("utf-8")("ignore"))
    return found, rest


class Label(unittest.TestCase):
    def test_nombre_y_carpeta_de_arriba_con_virgulilla(self):
        self.assertEqual(recent.label("/Users/ana/src/web", "/Users/ana"), ("web", "~/src"))
        self.assertEqual(recent.label("/Volumes/Apps/api", "/Users/ana"), ("api", "/Volumes/Apps"))
        self.assertEqual(recent.label("/Users/anabel/x", "/Users/ana"), ("x", "/Users/anabel"))
        self.assertEqual(recent.label("/Users/ana", "/Users/ana"), ("~", ""))
        self.assertEqual(recent.label("/tmp", "/Users/ana"), ("tmp", "/"))

    def test_busqueda_difusa(self):
        self.assertEqual(recent.match("vkt", "vkTERM  /Volumes"), [0, 1, 2])
        self.assertEqual(recent.match("V T", "vkTERM"), [0, 2])
        self.assertEqual(recent.match("", "vkTERM"), [])
        self.assertIsNone(recent.match("zz", "vkTERM"))


class Keys(unittest.TestCase):
    def test_clasicas_y_kitty(self):
        found, rest = keys(b"\x1b[A\x1b[B\r\x7f\x1b[13;2u\x1b[27u\x03\x1b[99;5u\x1b[34;9~")
        self.assertEqual([k for k, _ in found],
                         ["up", "down", "enter", "backspace", "window", "cancel", "cancel", "cancel", "cancel"])
        self.assertEqual(rest, b"")

    def test_texto_utf8_y_secuencias_partidas(self):
        found, rest = keys("añ".encode() + b"\x1b[13;")
        self.assertEqual(found, [("text", "a"), ("text", "ñ")])
        self.assertEqual(rest, b"\x1b[13;")
        self.assertEqual(keys(b"\x1b")[1], b"\x1b")
        self.assertEqual(keys(b"\x1bOA")[0], [("up", "")])
        self.assertEqual(keys(b"\x1b[200~")[0], [("ignore", "")])


class Picking(unittest.TestCase):
    FOLDERS = [("/Users/ana/src/web", ""), ("/Users/ana/src/api", "open"), ("/Volumes/USB/viejo", "missing")]

    def picker(self, window_only=False):
        return recent.Picker(list(self.FOLDERS), "/Users/ana", window_only)

    def test_enter_aqui_y_mayus_enter_ventana_nueva(self):
        p = self.picker()
        self.assertEqual(p.key("enter"), ("here", "/Users/ana/src/web"))
        p.key("down")
        self.assertEqual(p.key("window"), ("window", "/Users/ana/src/api"))
        self.assertEqual(self.picker(window_only=True).key("enter"), ("window", "/Users/ana/src/web"))
        self.assertEqual(p.key("cancel"), "cancel")

    def test_filtrar_y_moverse(self):
        p = self.picker()
        for ch in "api":
            p.key("text", ch)
        self.assertEqual([i for i, _ in p.rows], [1])
        p.key("backspace")
        p.key("backspace")
        self.assertEqual(p.query, "a")
        p.key("clear")
        self.assertEqual([i for i, _ in p.rows], [0, 1, 2])
        p.key("pagedown")
        self.assertEqual(p.selected, 2)
        p.key("up")
        self.assertEqual(p.selected, 1)

    def test_una_carpeta_que_no_existe_no_se_abre(self):
        p = self.picker()
        p.key("pagedown")
        self.assertIsNone(p.key("enter"))
        self.assertEqual(p.message, recent.MISSING)
        self.assertIn(recent.MISSING, p.render(100)[-1])
        p.key("up")
        self.assertEqual(p.message, "")

    def test_dibujo(self):
        lines = self.picker().render(60)
        self.assertEqual(len(lines), 3 + 2)
        self.assertIn(recent.PLACEHOLDER, lines[0])
        self.assertTrue(lines[1].startswith("\x1b[7m"))
        self.assertIn("abierta", lines[2])
        self.assertIn("no disponible", lines[3])
        self.assertIn("⇧Enter", lines[-1])
        plain = lambda line: __import__("re").sub(r"\x1b\[[0-9;]*m", "", line)
        self.assertTrue(all(len(plain(line)) <= 60 for line in lines))
        self.assertNotIn("⇧Enter", self.picker(window_only=True).render(60)[-1])


class Terminal(unittest.TestCase):
    """La orden de verdad en un pseudoterminal, con un estado propio."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="vkterm-recent-"))
        self.state = self.tmp / "state"
        for name in ("uno", "dos"):
            folder = self.tmp / name
            folder.mkdir()
            subprocess.run([str(ROOT / "bin" / "vkterm-state"), "dir", str(folder)], check=True,
                           capture_output=True, env=dict(os.environ, VKTERM_STATE_DIR=str(self.state)))
            time.sleep(1.1)  # la fecha de uso es la del archivo más reciente

    def tearDown(self):
        subprocess.run(["rm", "-rf", str(self.tmp)])

    def run_picker(self, *typed, args=()):
        out = self.tmp / "out"
        pid, fd = pty.fork()
        if pid == 0:
            os.environ["VKTERM_STATE_DIR"] = str(self.state)
            with open(out, "w") as f:
                os.dup2(f.fileno(), 1)
            os.execv(str(RECENT), [str(RECENT), *args])
        screen = bytearray()

        def drain():
            # Al terminar la orden, el pseudoterminal da fin de archivo (o EIO).
            try:
                while select.select([fd], [], [], 0.1)[0]:
                    chunk = os.read(fd, 65536)
                    if not chunk:
                        return
                    screen.extend(chunk)
            except OSError:
                pass
        for data in typed:
            time.sleep(0.4)
            drain()
            try:
                os.write(fd, data)
            except OSError:
                break  # ya terminó
        deadline = time.time() + 10
        while True:
            done, status = os.waitpid(pid, os.WNOHANG)
            if done:
                break
            drain()
            if time.time() > deadline:
                os.kill(pid, 9)
        drain()
        os.close(fd)
        return os.waitstatus_to_exitcode(status), out.read_text(), bytes(screen)

    def test_la_ultima_usada_primero_y_enter(self):
        code, out, screen = self.run_picker(b"\r")
        self.assertEqual((code, out), (0, "here\t%s\n" % (self.tmp / "dos").resolve()))
        self.assertIn(b"\x1b[>1u", screen)
        self.assertTrue(screen.rstrip(b"\r\n").endswith(b"\x1b[J\x1b[?25h\x1b[<u"), screen[-40:])

    def test_filtrar_y_mayus_enter(self):
        code, out, _ = self.run_picker(b"un", b"\x1b[13;2u")
        self.assertEqual((code, out), (0, "window\t%s\n" % (self.tmp / "uno").resolve()))

    def test_esc_clasico_cancela(self):
        code, out, _ = self.run_picker(b"\x1b")
        self.assertEqual((code, out), (1, ""))


if __name__ == "__main__":
    unittest.main()
