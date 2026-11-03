#!/usr/bin/env python3
"""Tests de bin/vkterm-git (python3 -m unittest scripts/test_vkterm_git.py).

Lo que usa la vista Control de código fuente: el estado (con la comparación
de conflictos con otra rama), preparar y quitar, descartar, borrar archivos
nuevos, commit y abortar una fusión, en repositorios temporales."""
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VKTERM_GIT = ROOT / "bin" / "vkterm-git"
ENV = dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@t", GIT_COMMITTER_NAME="t",
           GIT_COMMITTER_EMAIL="t@t", GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1")


def git(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, env=ENV, check=True).stdout


def vkterm_git(cwd, *args):
    return subprocess.run([str(VKTERM_GIT), *args], cwd=cwd, capture_output=True, text=True, env=ENV,
                          stdin=subprocess.DEVNULL)


def status(cwd, *args):
    """(cabecera clave → [valores], registros de git status)."""
    out = vkterm_git(cwd, "status", *args).stdout
    records = out.split("\0")
    header, rest = {}, []
    for i, record in enumerate(records):
        if record == "--":
            rest = [r for r in records[i + 1:] if r]
            break
        if record:
            key, _, value = record.partition("=")
            header.setdefault(key, []).append(value)
    return header, rest


class Repo(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name).resolve() / "repo"
        self.repo.mkdir()
        git(self.repo, "init", "-q", "-b", "main")
        self.write("a.txt", "uno\ndos\ntres\n")
        self.write("src/b.txt", "b\n")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "inicio")

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, name, text):
        path = self.repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def diverge(self):
        """`rama` y `main` cambian la misma línea de a.txt."""
        git(self.repo, "checkout", "-q", "-b", "rama")
        self.write("a.txt", "uno\nDOS rama\ntres\n")
        git(self.repo, "commit", "-q", "-am", "rama")
        git(self.repo, "checkout", "-q", "main")
        self.write("a.txt", "uno\nDOS main\ntres\n")
        git(self.repo, "commit", "-q", "-am", "main")
        git(self.repo, "checkout", "-q", "rama")


class Status(Repo):
    def test_sin_repositorio(self):
        with tempfile.TemporaryDirectory() as plain:
            header, rest = status(plain)
            self.assertEqual(header, {"repo": ["0"]})
            self.assertEqual(rest, [])

    def test_raiz_prefijo_y_cambios(self):
        self.write("a.txt", "cambiado\n")
        self.write("nuevo dir/x y.txt", "x\n")
        header, rest = status(self.repo / "src")
        self.assertEqual(header["root"], [str(self.repo)])
        self.assertEqual(header["prefix"], ["src/"])
        self.assertEqual(header["merge"], ["none"])  # en main, comparada con main
        self.assertIn("# branch.head main", rest)
        self.assertTrue(any(r.startswith("1 .M ") and r.endswith(" a.txt") for r in rest))
        self.assertIn("? nuevo dir/x y.txt", rest)

    def test_conflictos_con_main_sin_tocar_nada(self):
        self.diverge()
        header, _ = status(self.repo)
        self.assertEqual(header["target"], ["main"])
        self.assertEqual(header["merge"], ["conflicts"])
        self.assertEqual(header["conflict"], ["a.txt"])
        self.assertEqual((header["ahead"], header["behind"]), (["1"], ["1"]))
        merged = git(self.repo, "show", "%s:a.txt" % header["tree"][0])
        self.assertIn("<<<<<<< HEAD", merged)
        self.assertIn(">>>>>>> main", merged)
        self.assertFalse((self.repo / ".git" / "MERGE_HEAD").exists())
        self.assertEqual((self.repo / "a.txt").read_text(), "uno\nDOS rama\ntres\n")
        # Los mismos commits: no se vuelve a calcular.
        again, _ = status(self.repo, "", header["check"][0])
        self.assertEqual(again["merge"], ["same"])

    def test_otra_rama_sin_conflictos_y_rama_que_no_existe(self):
        git(self.repo, "checkout", "-q", "-b", "otra")
        self.write("c.txt", "c\n")
        git(self.repo, "add", "c.txt")
        git(self.repo, "commit", "-q", "-m", "c")
        git(self.repo, "checkout", "-q", "main")
        header, _ = status(self.repo, "otra")
        self.assertEqual((header["target"], header["merge"], header["behind"]), (["otra"], ["clean"], ["1"]))
        self.assertEqual(status(self.repo, "no-existe")[0]["merge"], ["missing"])

    def test_fusion_en_curso_con_su_mensaje(self):
        self.diverge()
        subprocess.run(["git", "-C", str(self.repo), "merge", "main"], capture_output=True, env=ENV)
        header, rest = status(self.repo)
        self.assertEqual(header["operation"], ["merge"])
        self.assertEqual(header["merge_msg"], ["Merge branch 'main' into rama"])
        self.assertTrue(any(r.startswith("u UU ") and r.endswith(" a.txt") for r in rest))
        self.assertEqual(vkterm_git(self.repo, "abort").returncode, 0)
        self.assertNotIn("operation", {k for k, v in status(self.repo)[0].items() if v != [""]})

    def test_rama_sin_commits(self):
        with tempfile.TemporaryDirectory() as tmp:
            git(tmp, "init", "-q", "-b", "main")
            Path(tmp, "a").write_text("a\n")
            header, rest = status(tmp)
            self.assertEqual(header["merge"], ["none"])
            self.assertIn("# branch.oid (initial)", rest)
            self.assertEqual(vkterm_git(tmp, "stage", "a").returncode, 0)
            self.assertEqual(git(tmp, "diff", "--cached", "--name-only").split(), ["a"])
            self.assertEqual(vkterm_git(tmp, "unstage", "a").returncode, 0)
            self.assertEqual(git(tmp, "diff", "--cached", "--name-only").split(), [])

    def test_ramas_locales_y_remotas_sin_head(self):
        clone = Path(self.tmp.name) / "clone"
        subprocess.run(["git", "clone", "-q", str(self.repo), str(clone)], check=True, env=ENV)
        out = vkterm_git(clone, "branches").stdout.splitlines()
        self.assertIn("local\tmain", out)
        self.assertIn("remote\torigin/main", out)
        self.assertFalse(any(line.endswith("/HEAD") for line in out))


class Actions(Repo):
    def staged(self):
        return git(self.repo, "diff", "--cached", "--name-only").split()

    def test_preparar_quitar_y_rutas_literales(self):
        self.write("[x].txt", "x\n")
        self.write("x.txt", "x\n")
        self.assertEqual(vkterm_git(self.repo, "stage", "[x].txt").returncode, 0)
        self.assertEqual(self.staged(), ["[x].txt"])  # no es un patrón
        (self.repo / "src" / "b.txt").unlink()
        vkterm_git(self.repo, "stage", "src/b.txt")
        self.assertEqual(sorted(self.staged()), ["[x].txt", "src/b.txt"])
        vkterm_git(self.repo, "unstage-all")
        self.assertEqual(self.staged(), [])
        vkterm_git(self.repo, "stage-all")
        self.assertEqual(sorted(self.staged()), ["[x].txt", "src/b.txt", "x.txt"])

    def test_descartar_y_borrar_nuevos(self):
        self.write("a.txt", "otro\n")
        self.write("nuevo.txt", "n\n")
        self.assertEqual(vkterm_git(self.repo, "discard", "a.txt").returncode, 0)
        self.assertEqual((self.repo / "a.txt").read_text(), "uno\ndos\ntres\n")
        self.assertEqual(vkterm_git(self.repo, "clean", "nuevo.txt").returncode, 0)
        self.assertFalse((self.repo / "nuevo.txt").exists())

    def test_commit_y_commit_de_todo(self):
        self.write("a.txt", "otro\n")
        self.write("nuevo.txt", "n\n")
        vkterm_git(self.repo, "stage", "a.txt")
        done = vkterm_git(self.repo, "commit", "mensaje con espacios")
        self.assertEqual(done.returncode, 0)
        self.assertTrue(done.stdout.strip().endswith("mensaje con espacios"))
        self.assertEqual(git(self.repo, "status", "--porcelain").strip(), "?? nuevo.txt")
        self.assertEqual(vkterm_git(self.repo, "commit", "--all", "todo").returncode, 0)
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")
        self.assertNotEqual(vkterm_git(self.repo, "commit", "").returncode, 0)


if __name__ == "__main__":
    unittest.main()
