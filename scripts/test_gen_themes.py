#!/usr/bin/env python3
"""Tests de scripts/gen-themes.py (python3 -m unittest scripts/test_gen_themes.py)."""
import importlib.util
import json
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("gen_themes", HERE / "gen-themes.py")
gen = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gen)

MANIFEST = json.loads((gen.THEMES / "themes.json").read_text(encoding="utf-8"))


def theme(theme_id):
    return gen.Theme(next(t for t in MANIFEST["themes"] if t["id"] == theme_id))


class Jsonc(unittest.TestCase):
    def test_comentarios_y_comas_finales(self):
        text = '{\n  // comentario\n  "a": "http://x", /* otro */\n  "b": [1, 2,],\n}'
        self.assertEqual(json.loads(gen.strip_jsonc(text)), {"a": "http://x", "b": [1, 2]})

    def test_include_mezcla_colores_y_reglas(self):
        data = gen.load_vscode(gen.SOURCES / "microsoft/dark_plus.json")
        # editor.background viene de dark_vs.json; las reglas de Dark+ van detrás.
        self.assertEqual(data["colors"]["editor.background"].upper(), "#1E1E1E")
        self.assertGreater(len(data["tokenColors"]), 40)


class Colores(unittest.TestCase):
    def test_hex_cortos_y_con_transparencia(self):
        self.assertEqual(gen.Color.parse("#888").hex(), "#888888")
        c = gen.Color.parse("#ffffff80")
        self.assertAlmostEqual(c.a, 128 / 255)
        self.assertEqual(c.over(gen.BLACK).hex(), "#808080")
        self.assertIsNone(gen.Color.parse("rojo"))

    def test_contraste(self):
        self.assertAlmostEqual(gen.BLACK.contrast(gen.WHITE), 21, places=0)


class DarkPlus(unittest.TestCase):
    """Valores de referencia de VS Code 1.143.1 con Dark+."""

    @classmethod
    def setUpClass(cls):
        cls.t = theme("dark-plus")
        cls.micro = cls.t.micro()

    def test_resaltado(self):
        self.assertEqual(self.micro["statement"], "#c586c0")         # keyword.control
        self.assertEqual(self.micro["identifier.var"], "#9cdcfe")    # variable
        self.assertEqual(self.micro["constant.string"], "#ce9178")   # string
        self.assertEqual(self.micro["comment"], "#6a9955")
        self.assertEqual(self.micro["type"], "#4ec9b0")

    def test_valores_por_defecto_de_vs_code(self):
        ui = self.t.ui()
        self.assertEqual(ui["sidebar_bg"].hex(), "#252526")
        self.assertEqual(ui["status_bg"].hex(), "#007acc")
        self.assertEqual(ui["selection_bg"].hex(), "#04395e")
        self.assertEqual(self.t.bg.hex(), "#1e1e1e")

    def test_paleta_del_terminal(self):
        self.assertEqual([c.hex() for c in self.t.ansi()][:2], ["#000000", "#cd3131"])

    def test_texto_y_fondo_los_pone_el_terminal(self):
        self.assertEqual(self.micro["default"], "default,default")


class Resolucion(unittest.TestCase):
    def test_gana_el_prefijo_mas_largo_y_despues_la_ultima_regla(self):
        tokens = gen.Tokens([
            {"scope": "keyword", "settings": {"foreground": "#111111"}},
            {"scope": "keyword.control", "settings": {"foreground": "#222222"}},
            {"scope": "keyword", "settings": {"foreground": "#333333", "fontStyle": "italic"}},
            {"scope": "source.go keyword", "settings": {"foreground": "#444444"}},
        ])
        self.assertEqual(tokens.lookup("keyword.control.flow", "foreground"), "#222222")
        self.assertEqual(tokens.lookup("keyword.other", "foreground"), "#333333")
        self.assertEqual(tokens.lookup("keyword.control", "fontStyle"), "italic")
        self.assertIsNone(tokens.lookup("string", "foreground"))


class TodosLosTemas(unittest.TestCase):
    def test_bordes_visibles_y_el_del_foco_destaca(self):
        for entry in MANIFEST["themes"]:
            t = gen.Theme(entry)
            ui = t.ui()
            self.assertGreaterEqual(ui["focus"].contrast(t.bg), 2.0, entry["id"])
            self.assertGreaterEqual(ui["border"].contrast(t.bg), 1.3, entry["id"])
            if not t.kind.startswith("hc"):
                self.assertLessEqual(ui["border"].contrast(t.bg), ui["focus"].contrast(t.bg), entry["id"])

    def test_ids_unicos_y_predeterminado(self):
        ids = [t["id"] for t in MANIFEST["themes"]]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(ids[0], MANIFEST["default"])

    def test_lo_generado_esta_al_dia(self):
        for path, text in gen.outputs().items():
            self.assertTrue(path.exists(), path)
            self.assertEqual(path.read_text(encoding="utf-8"), text, f"{path}: ejecuta scripts/gen-themes.py")


if __name__ == "__main__":
    unittest.main()
