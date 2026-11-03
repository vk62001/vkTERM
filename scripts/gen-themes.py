#!/usr/bin/env python3
"""vkTERM — genera los temas de color a partir de los temas de VS Code.

    scripts/gen-themes.py            escribe los archivos generados
    scripts/gen-themes.py --check    falla si alguno no está al día (CI)
    scripts/gen-themes.py --verify   descarga las fuentes (SOURCES.json) y
                                     comprueba que coinciden (sha256)

Entrada:
  config/themes/themes.json          los temas de ⌘K ⌘T: id, nombre, tipo,
                                     archivo de VS Code y colores del atenuado
  config/themes/vscode/              los temas de VS Code tal cual (licencias
                                     y procedencia en SOURCES.json), más
                                     vkterm/monokai-pro.json, hecho a mano

Salida (una por tema, con el nombre vkterm-<id>):
  config/ghostty/themes/vkterm-<id>          paleta del terminal, fondo y texto
  config/micro/colorschemes/vkterm-<id>.micro  resaltado del código e interfaz
                                             del editor, en color de 24 bits
  config/zellij/themes/vkterm.kdl            interfaz de Zellij y del plugin
                                             (barra lateral, listas, barra de
                                             estado, marcos)
  plugins/vkterm/src/themes_data.rs          la tabla de temas del plugin
  config/themes/generated.json               colores resueltos (pruebas)

Cada color que el tema no define toma el valor por defecto de VS Code para su
tipo (oscuro, claro o alto contraste), copiado de su registro de colores
(src/vs/platform/theme/common/colors, src/vs/workbench/common/theme.ts y
terminalColorRegistry.ts, VS Code 1.143.1, licencia MIT). El resaltado se
resuelve como en VS Code: gana la regla de `tokenColors` cuyo ámbito es el
prefijo más largo y, si empatan, la última.

Solo usa la biblioteca estándar de Python.
"""
import argparse
import hashlib
import io
import json
import re
import sys
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
THEMES = ROOT / "config/themes"
SOURCES = THEMES / "vscode"
GHOSTTY = ROOT / "config/ghostty/themes"
MICRO = ROOT / "config/micro/colorschemes"
ZELLIJ = ROOT / "config/zellij/themes/vkterm.kdl"
RUST = ROOT / "plugins/vkterm/src/themes_data.rs"
GENERATED = THEMES / "generated.json"

KINDS = {"dark": "dark", "light": "light", "hc-dark": "hcDark", "hc-light": "hcLight"}


# ------------------------------------------------------------------ JSONC

def strip_jsonc(text):
    """JSON con comentarios (// y /* */) y comas finales → JSON."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append(text[i:j + 1])
            i = j + 1
        elif text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
        elif text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = n if end < 0 else end + 2
        else:
            out.append(c)
            i += 1
    return re.sub(r",(\s*[}\]])", r"\1", "".join(out))


def load_vscode(path):
    """Un tema de VS Code con sus `include` resueltos: los colores del hijo
    sustituyen a los del padre y sus reglas de resaltado van detrás."""
    data = json.loads(strip_jsonc(path.read_text(encoding="utf-8")))
    if "include" in data:
        parent = load_vscode((path.parent / data["include"]).resolve())
        data["colors"] = {**parent.get("colors", {}), **data.get("colors", {})}
        data["tokenColors"] = parent.get("tokenColors", []) + data.get("tokenColors", [])
    if not isinstance(data.get("tokenColors", []), list):
        raise SystemExit(f"{path}: tokenColors en .tmTheme no está soportado")
    return data


# ----------------------------------------------------------------- colores

class Color:
    """RGBA; el canal alfa entre 0 y 1."""

    def __init__(self, r, g, b, a=1.0):
        self.r, self.g, self.b, self.a = r, g, b, a

    @staticmethod
    def parse(value):
        if not isinstance(value, str):
            return None
        h = value.strip().lstrip("#")
        if not re.fullmatch(r"[0-9a-fA-F]{3,8}", h) or len(h) not in (3, 4, 6, 8):
            return None
        if len(h) in (3, 4):
            h = "".join(c * 2 for c in h)
        a = int(h[6:8], 16) / 255 if len(h) == 8 else 1.0
        return Color(int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)

    def transparent(self, factor):
        return Color(self.r, self.g, self.b, self.a * factor)

    def over(self, background):
        """Opaco: mezclado sobre `background` según su transparencia."""
        if self.a >= 1 or background is None:
            return Color(self.r, self.g, self.b)
        a = self.a
        return Color(*(round(c * a + b * (1 - a)) for c, b in
                       ((self.r, background.r), (self.g, background.g), (self.b, background.b))))

    def mix(self, other, t):
        """t = 0 → este color; t = 1 → `other`."""
        return Color(*(round(a + (b - a) * t) for a, b in
                       ((self.r, other.r), (self.g, other.g), (self.b, other.b))))

    def luminance(self):
        def channel(c):
            c /= 255
            return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4
        return 0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)

    def contrast(self, other):
        a, b = sorted((self.luminance(), other.luminance()), reverse=True)
        return (a + 0.05) / (b + 0.05)

    def hex(self):
        return "#{:02x}{:02x}{:02x}".format(round(self.r), round(self.g), round(self.b))

    def rgb(self):
        return f"{round(self.r)} {round(self.g)} {round(self.b)}"


BLACK, WHITE = Color(0, 0, 0), Color(255, 255, 255)


# ------------------------------------------------ valores por defecto de VS Code
#
# Un valor es: "#hex", None, ("ref", clave), ("t", valor, factor) para
# transparent(), ("ifdef", clave, entonces, si no), o un dict por tipo de tema
# (dark, light, hcDark, hcLight).

def ref(key):
    return ("ref", key)


def t(value, factor):
    return ("t", value, factor)


def kinds(dark, light, hc_dark, hc_light):
    return {"dark": dark, "light": light, "hcDark": hc_dark, "hcLight": hc_light}


DEFAULTS = {
    # baseColors.ts
    "foreground": kinds("#CCCCCC", "#616161", "#FFFFFF", "#292929"),
    "errorForeground": kinds("#F48771", "#A1260D", "#F48771", "#B5200D"),
    "descriptionForeground": kinds(t(ref("foreground"), 0.7), "#717171",
                                   t(ref("foreground"), 0.7), t(ref("foreground"), 0.7)),
    "focusBorder": kinds("#007FD4", "#0090F1", "#F38518", "#006BBD"),
    "contrastBorder": kinds(None, None, "#6FC3DF", "#0F4A85"),
    "contrastActiveBorder": kinds(None, None, ref("focusBorder"), ref("focusBorder")),
    "textLink.foreground": kinds("#3794FF", "#006AB1", "#21A6FF", "#0F4A85"),
    # inputColors.ts
    "input.background": kinds("#3C3C3C", "#FFFFFF", "#000000", "#FFFFFF"),
    "input.foreground": ref("foreground"),
    "input.border": kinds(None, None, ref("contrastBorder"), ref("contrastBorder")),
    "input.placeholderForeground": kinds(t(ref("foreground"), 0.5), t(ref("foreground"), 0.5),
                                         t(ref("foreground"), 0.7), t(ref("foreground"), 0.7)),
    "button.background": kinds("#0E639C", "#007ACC", "#000000", "#0F4A85"),
    "button.foreground": "#FFFFFF",
    "keybindingLabel.foreground": kinds("#CCCCCC", "#555555", "#FFFFFF", ref("foreground")),
    "keybindingLabel.background": kinds("#8080802B", "#DDDDDD66", "#00000000", "#00000000"),
    # editorColors.ts
    "editor.background": kinds("#1E1E1E", "#FFFFFF", "#000000", "#FFFFFF"),
    "editor.foreground": kinds("#BBBBBB", "#333333", "#FFFFFF", ref("foreground")),
    "editorWidget.background": kinds("#252526", "#F3F3F3", "#0C141F", "#FFFFFF"),
    "editorWidget.foreground": ref("foreground"),
    "editorWidget.border": kinds(t(ref("editorWidget.foreground"), 0.2), t(ref("editorWidget.foreground"), 0.2),
                                 ref("contrastBorder"), ref("contrastBorder")),
    "editorError.foreground": kinds("#F14C4C", "#E51400", "#F48771", "#B5200D"),
    "editorWarning.foreground": kinds("#CCA700", "#BF8803", "#FFD370", "#895503"),
    "editorInfo.foreground": kinds("#59A4F9", "#0063D3", "#59A4F9", "#0063D3"),
    "editor.selectionBackground": kinds("#264F78", "#ADD6FF", "#F3F518", "#0F4A85"),
    "editor.selectionForeground": kinds(None, None, "#000000", "#FFFFFF"),
    "editor.findMatchHighlightBackground": kinds("#EA5C0055", "#EA5C0055", None, None),
    "scrollbarSlider.background": kinds(t("#797979", 0.4), t("#646464", 0.4),
                                        t(ref("contrastBorder"), 0.6), t(ref("contrastBorder"), 0.4)),
    # editorColorRegistry.ts
    "editor.lineHighlightBackground": None,
    "editor.lineHighlightBorder": kinds("#282828", "#EEEEEE", "#F38518", ref("contrastBorder")),
    "editorCursor.foreground": kinds("#AEAFAD", "#000000", "#FFFFFF", "#0F4A85"),
    "editorWhitespace.foreground": kinds("#E3E4E229", "#33333333", "#E3E4E229", "#CCCCCC"),
    "editorIndentGuide.background": ref("editorWhitespace.foreground"),
    "editorIndentGuide.background1": ref("editorIndentGuide.background"),
    "editorLineNumber.foreground": kinds("#858585", "#237893", "#FFFFFF", "#292929"),
    "editorActiveLineNumber.foreground": kinds("#C6C6C6", "#0B216F", ref("contrastActiveBorder"),
                                               ref("contrastActiveBorder")),
    "editorLineNumber.activeForeground": ref("editorActiveLineNumber.foreground"),
    "editorRuler.foreground": kinds("#5A5A5A", "#D3D3D3", "#FFFFFF", "#292929"),
    "editorBracketMatch.background": kinds("#0064001A", "#0064001A", "#0064001A", "#0000"),
    "editorBracketMatch.border": kinds("#888888", "#B9B9B9", ref("contrastBorder"), ref("contrastBorder")),
    # scm/common/quickDiff.ts
    "editorGutter.modifiedBackground": kinds("#1B81A8", "#2090D3", "#1B81A8", "#2090D3"),
    "editorGutter.addedBackground": kinds("#487E02", "#48985D", "#487E02", "#48985D"),
    "editorGutter.deletedBackground": ref("editorError.foreground"),
    # listColors.ts y quickpickColors.ts
    "list.activeSelectionBackground": kinds("#04395E", "#0060C0", None, t("#0F4A85", 0.1)),
    "list.activeSelectionForeground": kinds("#FFFFFF", "#FFFFFF", None, None),
    "list.inactiveSelectionBackground": kinds("#37373D", "#E4E6F1", None, t("#0F4A85", 0.1)),
    "list.inactiveSelectionForeground": None,
    "list.highlightForeground": kinds("#2AAAFF", "#0066BF", ref("focusBorder"), ref("focusBorder")),
    "list.focusHighlightForeground": kinds(ref("list.highlightForeground"),
                                           ("ifdef", "list.activeSelectionBackground",
                                            ref("list.highlightForeground"), "#BBE7FF"),
                                           ref("list.highlightForeground"), ref("list.highlightForeground")),
    "quickInput.background": ref("editorWidget.background"),
    "quickInput.foreground": ref("editorWidget.foreground"),
    "quickInputList.focusBackground": kinds(ref("list.activeSelectionBackground"),
                                            ref("list.activeSelectionBackground"), None, None),
    "quickInputList.focusForeground": ref("list.activeSelectionForeground"),
    "pickerGroup.foreground": kinds("#3794FF", "#0066BF", "#FFFFFF", "#0F4A85"),
    # workbench/common/theme.ts
    "activityBarBadge.background": kinds("#007ACC", "#007ACC", "#000000", "#0F4A85"),
    "activityBarBadge.foreground": "#FFFFFF",
    "statusBarItem.remoteBackground": ref("activityBarBadge.background"),
    "statusBarItem.remoteForeground": ref("activityBarBadge.foreground"),
    "statusBar.background": kinds("#007ACC", "#007ACC", None, None),
    "statusBar.foreground": kinds("#FFFFFF", "#FFFFFF", "#FFFFFF", ref("editor.foreground")),
    "statusBar.border": kinds(None, None, ref("contrastBorder"), ref("contrastBorder")),
    "sideBar.background": kinds("#252526", "#F3F3F3", "#000000", "#FFFFFF"),
    "sideBar.foreground": None,
    "sideBarSectionHeader.background": kinds(t("#808080", 0.2), t("#808080", 0.2), None, None),
    "sideBarSectionHeader.foreground": ref("sideBar.foreground"),
    "tab.activeBackground": ref("editor.background"),
    "tab.inactiveBackground": kinds("#2D2D2D", "#ECECEC", None, None),
    "tab.activeForeground": kinds("#FFFFFF", "#333333", "#FFFFFF", "#292929"),
    "tab.inactiveForeground": kinds(t(ref("tab.activeForeground"), 0.5), t(ref("tab.activeForeground"), 0.7),
                                    "#FFFFFF", "#292929"),
    "editorGroupHeader.tabsBackground": kinds("#252526", "#F3F3F3", None, None),
    "editorGroup.border": kinds("#444444", "#E7E7E7", ref("contrastBorder"), ref("contrastBorder")),
    "panel.background": ref("editor.background"),
    "panel.border": kinds(t("#808080", 0.35), t("#808080", 0.35), ref("contrastBorder"), ref("contrastBorder")),
    # terminalColorRegistry.ts
    "terminal.background": None,
    "terminal.foreground": kinds("#CCCCCC", "#333333", "#FFFFFF", "#292929"),
    "terminalCursor.foreground": None,
    "terminal.selectionBackground": ref("editor.selectionBackground"),
    # extensions/git/package.json (colores de los archivos según su estado)
    "gitDecoration.addedResourceForeground": kinds("#81b88b", "#587c0c", "#a1e3ad", "#374e06"),
    "gitDecoration.modifiedResourceForeground": kinds("#E2C08D", "#895503", "#E2C08D", "#895503"),
    "gitDecoration.deletedResourceForeground": kinds("#c74e39", "#ad0707", "#c74e39", "#ad0707"),
    "gitDecoration.renamedResourceForeground": kinds("#73C991", "#007100", "#73C991", "#007100"),
    "gitDecoration.untrackedResourceForeground": kinds("#73C991", "#007100", "#73C991", "#007100"),
    "gitDecoration.ignoredResourceForeground": kinds("#8C8C8C", "#8E8E90", "#A7A8A9", "#8e8e90"),
    "gitDecoration.conflictingResourceForeground": kinds("#e4676b", "#ad0707", "#c74e39", "#ad0707"),
}

# Colores de git en la vista Control de código fuente y el explorador, en el
# orden de `git: [...]` en src/themes_data.rs (GitColors en src/themes.rs).
GIT_COLORS = ["modified", "added", "deleted", "renamed", "untracked", "ignored", "conflicting"]

ANSI = [
    ("terminal.ansiBlack", kinds("#000000", "#000000", "#000000", "#292929")),
    ("terminal.ansiRed", kinds("#CD3131", "#CD3131", "#CD0000", "#CD3131")),
    ("terminal.ansiGreen", kinds("#0DBC79", "#107C10", "#00CD00", "#136C13")),
    ("terminal.ansiYellow", kinds("#E5E510", "#949800", "#CDCD00", "#949800")),
    ("terminal.ansiBlue", kinds("#2472C8", "#0451A5", "#0000EE", "#0451A5")),
    ("terminal.ansiMagenta", kinds("#BC3FBC", "#BC05BC", "#CD00CD", "#BC05BC")),
    ("terminal.ansiCyan", kinds("#11A8CD", "#0598BC", "#00CDCD", "#0598BC")),
    ("terminal.ansiWhite", kinds("#E5E5E5", "#555555", "#E5E5E5", "#555555")),
    ("terminal.ansiBrightBlack", kinds("#666666", "#666666", "#7F7F7F", "#666666")),
    ("terminal.ansiBrightRed", kinds("#F14C4C", "#F14C4C", "#FF0000", "#CD3131")),
    ("terminal.ansiBrightGreen", kinds("#23D18B", "#14CE14", "#00FF00", "#00BC00")),
    ("terminal.ansiBrightYellow", kinds("#F5F543", "#B5BA00", "#FFFF00", "#B5BA00")),
    ("terminal.ansiBrightBlue", kinds("#3B8EEA", "#3B8EEA", "#5C5CFF", "#0451A5")),
    ("terminal.ansiBrightMagenta", kinds("#D670D6", "#D670D6", "#FF00FF", "#BC05BC")),
    ("terminal.ansiBrightCyan", kinds("#29B8DB", "#29B8DB", "#00FFFF", "#0598BC")),
    ("terminal.ansiBrightWhite", kinds("#E5E5E5", "#A5A5A5", "#FFFFFF", "#A5A5A5")),
]
DEFAULTS.update(ANSI)


class Colors:
    """Los colores de un tema, con los valores por defecto de VS Code."""

    def __init__(self, colors, kind):
        self.colors = colors
        self.kind = KINDS[kind]

    def get(self, key):
        """El color (con transparencia) o None si ni el tema ni VS Code lo definen."""
        if key in self.colors:
            return Color.parse(self.colors[key])
        return self.evaluate(DEFAULTS.get(key))

    def defined(self, key):
        return self.get(key) is not None

    def evaluate(self, value):
        if isinstance(value, dict):
            value = value[self.kind]
        if value is None:
            return None
        if isinstance(value, str):
            return Color.parse(value)
        op = value[0]
        if op == "ref":
            return self.get(value[1])
        if op == "t":
            color = self.evaluate(value[1])
            return color.transparent(value[2]) if color else None
        if op == "ifdef":
            return self.evaluate(value[2] if self.defined(value[1]) else value[3])
        raise ValueError(value)

    def first(self, *keys):
        for key in keys:
            color = self.get(key)
            if color is not None:
                return color
        return None


# ---------------------------------------------------------------- resaltado

class Tokens:
    """Las reglas de `tokenColors`, resueltas por ámbito como en VS Code."""

    def __init__(self, rules):
        self.rules = []  # (segmentos del selector, ajustes, orden)
        self.globals = {}
        for order, rule in enumerate(rules):
            settings = rule.get("settings", {})
            scopes = rule.get("scope")
            if scopes is None:
                self.globals.update(settings)
                continue
            if isinstance(scopes, str):
                scopes = scopes.split(",")
            for selector in scopes:
                selector = selector.strip()
                # Solo selectores de un ámbito: sin descendientes ni exclusiones.
                if selector and " " not in selector and not selector.startswith("-"):
                    self.rules.append((selector.split("."), settings, order))

    def lookup(self, scope, field):
        """El valor de `field` (foreground o fontStyle) para `scope`, o None."""
        parts = scope.split(".")
        best = None
        for selector, settings, order in self.rules:
            if field not in settings or parts[:len(selector)] != selector:
                continue
            key = (len(selector), order)
            if best is None or key > best[0]:
                best = (key, settings[field])
        return None if best is None else best[1]

    def style(self, scopes):
        """(color, estilo) del primer ámbito de la lista con color propio."""
        for scope in scopes:
            color = Color.parse(self.lookup(scope, "foreground"))
            if color is not None:
                return color, self.lookup(scope, "fontStyle") or ""
        return None, ""


# Grupo de Micro → ámbitos de TextMate, en orden. Micro asigna los grupos con
# expresiones regulares por lenguaje, más gruesas que las gramáticas de VS Code:
# el color es el del tema; qué palabra lo recibe puede no coincidir.
SYNTAX = [
    ("comment", ["comment"]),
    ("comment.bright", ["comment"]),
    ("constant", ["constant.language", "constant"]),
    ("constant.bool", ["constant.language.boolean", "constant.language"]),
    ("constant.number", ["constant.numeric"]),
    ("constant.string", ["string"]),
    ("constant.string.char", ["constant.character", "string"]),
    ("constant.string.url", ["markup.underline.link", "string"]),
    ("constant.specialChar", ["constant.character.escape", "constant.character"]),
    ("identifier", ["entity.name.function", "support.function"]),
    ("identifier.class", ["entity.name.type.class", "entity.name.type", "support.class"]),
    ("identifier.var", ["variable.other", "variable"]),
    ("identifier.macro", ["entity.name.function.preprocessor", "entity.name.function"]),
    ("statement", ["keyword.control", "keyword"]),
    ("special", ["keyword.control", "keyword"]),
    ("preproc", ["keyword.control.directive", "meta.preprocessor", "keyword.control", "keyword"]),
    ("preproc.shebang", ["comment"]),
    ("type", ["support.type", "entity.name.type", "storage.type"]),
    ("type.keyword", ["storage.type", "storage.modifier", "keyword"]),
    ("symbol", ["punctuation"]),
    ("symbol.operator", ["keyword.operator"]),
    ("symbol.brackets", ["punctuation"]),
    ("symbol.tag", ["entity.name.tag"]),
    ("symbol.tag.extended", ["entity.other.attribute-name"]),
    ("underlined", ["markup.underline", "markup.underline.link"]),
    ("error", ["invalid.illegal", "invalid"]),
]


def micro_style(fg=None, bg=None, extra=""):
    words = [w for w in ("bold", "italic", "underline") if w in extra]
    color = (fg.hex() if fg else "") + ("," + bg.hex() if bg else "")
    return " ".join(words + [color]).strip() or "default"


# ------------------------------------------------------------------ un tema

class Theme:
    def __init__(self, entry):
        self.id = entry["id"]
        self.name = entry["name"]
        self.kind = entry["kind"]
        self.dark = self.kind in ("dark", "hc-dark")
        self.dim = entry.get("dim", True)
        data = load_vscode(SOURCES / entry["source"])
        self.c = Colors(data.get("colors", {}), self.kind)
        self.tokens = Tokens(data.get("tokenColors", []))
        c = self.c
        globals_ = self.tokens.globals
        self.bg = (c.get("editor.background") or Color.parse(globals_.get("background"))).over(BLACK)
        self.fg = (c.get("editor.foreground") or Color.parse(globals_.get("foreground"))).over(self.bg)
        self.inactive = entry.get("inactive")

    def solid(self, color, over=None):
        return color.over(over or self.bg) if color is not None else None

    def pick(self, *keys, over=None):
        """El primer color definido entre `keys`, opaco sobre `over`."""
        return self.solid(self.c.first(*keys), over)

    def visible(self, keys, minimum, fallback, maximum=float("inf")):
        """El primer color de `keys` que se distingue del fondo del editor
        (contraste entre `minimum` y `maximum`). Algunos temas dejan el borde
        del foco o el de los paneles casi del color del fondo, o pintan los
        bordes más vivos que el foco: en vkTERM marcan qué panel tiene el foco
        y dónde empieza cada uno."""
        for key in keys:
            color = self.pick(key)
            if color is not None and minimum <= color.contrast(self.bg) < maximum:
                return color
        return fallback

    def focus(self):
        return self.visible(["focusBorder", "tab.activeBorderTop", "tab.activeBorder", "activityBar.activeBorder",
                             "list.highlightForeground", "textLink.foreground", "button.background"], 2.0, self.fg)

    # Interfaz, con los respaldos que usa VS Code cuando un color es None.
    def ui(self):
        c, bg, fg = self.c, self.bg, self.fg
        sidebar_bg = self.pick("sideBar.background", over=bg) or bg
        sidebar_fg = self.pick("sideBar.foreground", "foreground", over=sidebar_bg)
        quick_bg = self.pick("quickInput.background", over=bg) or sidebar_bg
        quick_fg = self.pick("quickInput.foreground", "foreground", over=quick_bg)
        status_bg = self.pick("statusBar.background", over=bg) or bg
        border = self.border()
        focus = self.focus()
        active_sel = self.pick("list.activeSelectionBackground", over=sidebar_bg)
        if active_sel is None:  # alto contraste: solo un borde; se usa el del foco
            active_sel = focus.mix(sidebar_bg, 0.6)
        inactive_sel = self.pick("list.inactiveSelectionBackground", over=sidebar_bg) or active_sel.mix(sidebar_bg, 0.5)
        quick_sel = self.pick("quickInputList.focusBackground", "list.activeSelectionBackground", over=quick_bg)
        if quick_sel is None:
            quick_sel = focus.mix(quick_bg, 0.6)
        header_bg = self.pick("sideBarSectionHeader.background", over=sidebar_bg) or sidebar_bg
        keycap_bg = self.pick("keybindingLabel.background", over=bg)
        return {
            "sidebar_fg": sidebar_fg,
            "sidebar_bg": sidebar_bg,
            "header_fg": self.pick("sideBarSectionHeader.foreground", "sideBarTitle.foreground", "sideBar.foreground",
                                   "foreground", over=header_bg),
            "header_bg": header_bg,
            "selection_fg": self.pick("list.activeSelectionForeground", "sideBar.foreground", "foreground",
                                      over=active_sel),
            "selection_bg": active_sel,
            "inactive_selection_fg": self.pick("list.inactiveSelectionForeground", "sideBar.foreground",
                                               "foreground", over=inactive_sel),
            "inactive_selection_bg": inactive_sel,
            "highlight": self.pick("list.highlightForeground", over=sidebar_bg),
            "focus_highlight": self.pick("list.focusHighlightForeground", "list.highlightForeground", over=active_sel),
            "description": self.pick("descriptionForeground", over=sidebar_bg),
            "quick_fg": quick_fg,
            "quick_bg": quick_bg,
            "quick_selection_fg": self.pick("quickInputList.focusForeground", "list.activeSelectionForeground",
                                            "foreground", over=quick_sel),
            "quick_selection_bg": quick_sel,
            "quick_highlight": self.pick("list.highlightForeground", over=quick_bg),
            "quick_description": self.pick("descriptionForeground", over=quick_bg),
            "keybinding": self.pick("keybindingLabel.foreground", "foreground", over=quick_bg),
            "keycap_fg": self.pick("keybindingLabel.foreground", "foreground", over=keycap_bg),
            "keycap_bg": keycap_bg,
            "picker_group": self.pick("pickerGroup.foreground", over=quick_bg),
            "status_fg": self.pick("statusBar.foreground", over=status_bg),
            "status_bg": status_bg,
            "status_item_fg": self.pick("statusBarItem.remoteForeground", over=status_bg),
            "status_item_bg": self.pick("statusBarItem.remoteBackground", over=status_bg),
            "tab_fg": self.pick("tab.inactiveForeground", over=self.pick("tab.inactiveBackground",
                                                                         "editorGroupHeader.tabsBackground") or bg),
            "tab_bg": self.pick("tab.inactiveBackground", "editorGroupHeader.tabsBackground") or bg,
            "tab_active_fg": self.pick("tab.activeForeground"),
            "tab_active_bg": self.pick("tab.activeBackground") or bg,
            "tabs_bg": self.pick("editorGroupHeader.tabsBackground", "tab.inactiveBackground") or bg,
            "border": border,
            "focus": focus,
            "warning": self.pick("editorWarning.foreground"),
            "error": self.pick("errorForeground", "editorError.foreground", over=sidebar_bg),
            "success": self.pick("terminal.ansiGreen"),
            **{"git_" + name: self.pick(f"gitDecoration.{name}ResourceForeground", over=sidebar_bg)
               for name in GIT_COLORS},
            "editor_fg": fg,
            "editor_bg": bg,
        }

    def border(self):
        """Borde de los paneles sin foco: visible, pero menos que el del foco
        (en alto contraste, el del tema: los dos se distinguen por el tono)."""
        limit = float("inf") if self.kind.startswith("hc") else min(self.focus().contrast(self.bg), 4.5)
        return self.visible(["editorGroup.border", "sideBar.border", "tab.border", "panel.border",
                             "contrastBorder"], 1.3, self.fg.mix(self.bg, 0.7), maximum=limit)

    def dim_colors(self):
        """Texto y fondo de los paneles sin foco: los del manifiesto o, si no,
        el fondo oscurecido y el texto a medio camino del fondo."""
        if self.inactive:
            return Color.parse(self.inactive["fg"]), Color.parse(self.inactive["bg"])
        bg = self.bg.mix(BLACK, 0.35 if self.dark else 0.075)
        return self.fg.mix(self.bg, 0.55 if self.dark else 0.45), bg

    def ansi(self):
        return [self.pick(key) for key, _ in ANSI]

    # Ghostty
    def ghostty(self):
        cursor = self.pick("terminalCursor.foreground", "editorCursor.foreground") or self.fg
        selection = self.pick("terminal.selectionBackground", "editor.selectionBackground")
        lines = [f"# {self.name}. Generado por scripts/gen-themes.py: no editar.",
                 f"background = {self.bg.hex()}",
                 f"foreground = {self.fg.hex()}",
                 f"cursor-color = {cursor.hex()}",
                 f"cursor-text = {self.bg.hex()}",
                 f"selection-background = {selection.hex()}"]
        selection_fg = self.pick("terminal.selectionForeground", "editor.selectionForeground", over=selection)
        if selection_fg is not None:
            lines.append(f"selection-foreground = {selection_fg.hex()}")
        lines += [f"palette = {i}={color.hex()}" for i, color in enumerate(self.ansi())]
        return "\n".join(lines) + "\n"

    # Micro
    def micro(self):
        c, bg, fg = self.c, self.bg, self.fg
        out = {"default": "default,default"}
        for group, scopes in SYNTAX:
            color, font = self.tokens.style(scopes)
            if color is not None:
                out[group] = micro_style(self.solid(color), extra=font)
        comment = self.tokens.style(["comment"])[0]
        out["todo"] = micro_style(self.solid(comment) if comment else fg, extra="bold")
        if "error" not in out:
            out["error"] = micro_style(self.pick("editorError.foreground"), extra="underline")
        if "underlined" not in out:
            out["underlined"] = micro_style(self.pick("textLink.foreground"), extra="underline")
        else:
            out["underlined"] = "underline " + out["underlined"].split()[-1]

        selection = self.pick("editor.selectionBackground")
        out["selection"] = micro_style(self.pick("editor.selectionForeground", over=selection) or fg, selection)
        # Micro no dibuja bordes: el de la línea del cursor (Dark+, Light+) se
        # usa como fondo solo si es discreto; en alto contraste no hay fondo.
        line = self.pick("editor.lineHighlightBackground")
        border = self.pick("editor.lineHighlightBorder")
        if line is None and border is not None and border.contrast(bg) < 1.8:
            line = border
        out["cursor-line"] = micro_style(bg=line) if line else "default"
        out["line-number"] = micro_style(self.pick("editorLineNumber.foreground"))
        out["current-line-number"] = micro_style(self.pick("editorLineNumber.activeForeground", "editor.foreground"))
        out["indent-char"] = micro_style(self.pick("editorIndentGuide.background1", "editorIndentGuide.background"))
        match = self.pick("editorBracketMatch.background")
        if match is None or match.contrast(bg) < 1.3:
            match = self.pick("editorBracketMatch.border") or self.pick("focusBorder")
        out["match-brace"] = micro_style(fg if fg.contrast(match) >= 3 else bg, match)
        find = self.pick("editor.findMatchHighlightBackground", "editor.selectionBackground")
        out["hlsearch"] = micro_style(fg if fg.contrast(find) >= 3 else bg, find)
        tabs_bg = self.pick("editorGroupHeader.tabsBackground", "tab.inactiveBackground") or bg
        tab_bg = self.pick("tab.inactiveBackground", "editorGroupHeader.tabsBackground") or bg
        out["tabbar"] = micro_style(self.pick("tab.inactiveForeground", over=tab_bg), tabs_bg)
        out["tabbar.active"] = micro_style(self.pick("tab.activeForeground"), self.pick("tab.activeBackground") or bg)
        out["statusline"] = micro_style(self.pick("tab.inactiveForeground", over=tabs_bg), tabs_bg)
        out["statusline.inactive"] = out["statusline"]
        out["statusline.suggestions"] = micro_style(self.pick("quickInput.foreground", "foreground"),
                                                    self.pick("quickInput.background"))
        out["divider"] = micro_style(self.border())
        out["scrollbar"] = micro_style(self.pick("scrollbarSlider.background"))
        ruler = self.pick("editorRuler.foreground")
        out["color-column"] = micro_style(bg=ruler.mix(bg, 0.7)) if ruler else "default"
        out["diff-added"] = micro_style(self.pick("editorGutter.addedBackground"))
        out["diff-modified"] = micro_style(self.pick("editorGutter.modifiedBackground"))
        out["diff-deleted"] = micro_style(self.pick("editorGutter.deletedBackground"))
        error, warning = self.pick("editorError.foreground"), self.pick("editorWarning.foreground")
        out["gutter-error"] = micro_style(error)
        out["gutter-warning"] = micro_style(warning)
        out["gutter-info"] = micro_style(self.pick("editorInfo.foreground"))
        out["error-message"] = micro_style(error)
        out["message"] = micro_style(fg)
        out["trailingws"] = micro_style(bg=error.mix(bg, 0.6))
        out["tab-error"] = out["trailingws"]
        return out

    def micro_text(self):
        lines = [f"# {self.name}. Generado por scripts/gen-themes.py: no editar.",
                 "# `default` sin colores: el texto y el fondo los pone el terminal",
                 "# (Ghostty), para que el atenuado de los paneles sin foco funcione."]
        lines += [f'color-link {group} "{style}"' for group, style in self.micro().items()]
        return "\n".join(lines) + "\n"

    # Zellij (formato por componentes, 0.43+)
    def zellij(self):
        u = self.ui()
        ansi = self.ansi()

        def block(name, base, background=None, e=(None, None, None, None)):
            rows = [f"base {base.rgb()}", f"background {background.rgb() if background else '0'}"]
            rows += [f"emphasis_{i} {(color or base).rgb()}" for i, color in enumerate(e)]
            return f"        {name} {{\n" + "".join(f"            {r}\n" for r in rows) + "        }\n"

        accents = (u["highlight"], u["description"], u["focus"], u["status_item_bg"])
        text = "".join([
            block("text_unselected", u["sidebar_fg"], u["sidebar_bg"], accents),
            block("text_selected", u["selection_fg"], u["selection_bg"],
                  (u["focus_highlight"], u["description"], u["focus"], u["status_item_bg"])),
            block("ribbon_selected", u["status_fg"], u["status_bg"],
                  (u["status_item_fg"], u["status_item_bg"], u["focus"], u["warning"])),
            block("ribbon_unselected", u["tab_fg"], u["tab_bg"],
                  (u["highlight"], u["tab_active_fg"], u["focus"], u["warning"])),
            block("table_title", u["header_fg"], u["header_bg"], accents),
            block("table_cell_selected", u["inactive_selection_fg"], u["inactive_selection_bg"], accents),
            # El editor: la ventana de atajos (⌘K ⌘S), como el editor «Keyboard
            # Shortcuts» de VS Code, y las teclas en etiquetas (keybindingLabel).
            block("table_cell_unselected", u["editor_fg"], u["editor_bg"],
                  (u["keycap_fg"], u["keycap_bg"], u["focus"], u["border"])),
            block("list_selected", u["quick_selection_fg"], u["quick_selection_bg"],
                  (u["focus_highlight"], u["keybinding"], u["quick_description"], u["picker_group"])),
            block("list_unselected", u["quick_fg"], u["quick_bg"],
                  (u["quick_highlight"], u["keybinding"], u["quick_description"], u["picker_group"])),
            block("frame_selected", u["focus"], None, (u["focus"], u["highlight"], u["warning"], u["border"])),
            block("frame_unselected", u["border"], None, (u["border"],) * 4),
            block("frame_highlight", u["warning"], None, (u["focus"], u["highlight"], u["warning"], u["warning"])),
            block("exit_code_success", u["success"], None, (ansi[6], u["sidebar_bg"], ansi[5], ansi[4])),
            block("exit_code_error", u["error"], None, (ansi[3], ansi[1], ansi[1], ansi[1])),
        ])
        players = "".join(f"            player_{i + 1} {ansi[9 + i % 6].rgb()}\n" for i in range(10))
        text += "        multiplayer_user_colors {\n" + players + "        }\n"
        return f"    vkterm-{self.id} {{\n{text}    }}\n"

    def summary(self):
        """Colores resueltos, para scripts/smoke-test.py."""
        fg, bg = self.dim_colors()
        return {
            "id": self.id,
            "name": self.name,
            "kind": self.kind,
            "dim": self.dim,
            "ghostty": f"vkterm-{self.id}",
            "zellij": f"vkterm-{self.id}",
            "micro": f"vkterm-{self.id}",
            "background": self.bg.hex(),
            "foreground": self.fg.hex(),
            "inactive_fg": fg.hex(),
            "inactive_bg": bg.hex(),
            "ui": {k: v.hex() for k, v in self.ui().items() if v is not None},
            "syntax": self.micro(),
        }


# ------------------------------------------------------------------ salida

def rust(themes, default):
    rows = []
    for theme in themes:
        fg, bg = theme.dim_colors()
        kind = {"dark": "Dark", "light": "Light", "hc-dark": "HighContrastDark", "hc-light": "HighContrastLight"}
        ui = theme.ui()
        git = ", ".join(f'"{ui["git_" + name].hex()}"' for name in GIT_COLORS)
        rows.append(
            "    Theme {\n"
            f"        id: {json.dumps(theme.id)},\n"
            f"        name: {json.dumps(theme.name)},\n"
            f"        kind: Kind::{kind[theme.kind]},\n"
            f"        dim: {'true' if theme.dim else 'false'},\n"
            f"        inactive_fg: \"{fg.hex()}\",\n"
            f"        inactive_bg: \"{bg.hex()}\",\n"
            f"        git: [{git}],\n"
            "    },\n")
    return ("// Generado por scripts/gen-themes.py a partir de config/themes/themes.json: no editar.\n"
            "use crate::themes::{Kind, Theme};\n\n"
            f"pub const DEFAULT: &str = {json.dumps(default)};\n\n"
            "pub const THEMES: &[Theme] = &[\n" + "".join(rows) + "];\n")


def outputs():
    manifest = json.loads((THEMES / "themes.json").read_text(encoding="utf-8"))
    themes = [Theme(entry) for entry in manifest["themes"]]
    files = {}
    for theme in themes:
        files[GHOSTTY / f"vkterm-{theme.id}"] = theme.ghostty()
        files[MICRO / f"vkterm-{theme.id}.micro"] = theme.micro_text()
    files[ZELLIJ] = ("// Temas de vkTERM, uno por tema de ⌘K ⌘T, con los colores de la interfaz\n"
                     "// de VS Code. Generado por scripts/gen-themes.py: no editar.\n"
                     "themes {\n" + "".join(t.zellij() for t in themes) + "}\n")
    files[RUST] = rust(themes, manifest["default"])
    files[GENERATED] = json.dumps({"default": manifest["default"], "themes": [t.summary() for t in themes]},
                                  indent=1, ensure_ascii=False) + "\n"
    return files


def stale_files(files):
    """Archivos generados que ya no corresponden a ningún tema."""
    keep = set(files)
    found = list(GHOSTTY.glob("vkterm-*")) + list(MICRO.glob("vkterm-*.micro"))
    return [p for p in found if p not in keep]


def verify():
    """Descarga cada fuente de SOURCES.json y compara su sha256."""
    sources = json.loads((SOURCES / "SOURCES.json").read_text())
    failed = 0
    cache = {}
    for rel, meta in sources.items():
        data = cache.get(meta["url"])
        if data is None:
            with urllib.request.urlopen(meta["url"], timeout=60) as response:
                data = cache[meta["url"]] = response.read()
        if "member" in meta:
            data = zipfile.ZipFile(io.BytesIO(data)).read(meta["member"])
        ok = hashlib.sha256(data).hexdigest() == meta["sha256"] == hashlib.sha256((SOURCES / rel).read_bytes()).hexdigest()
        print(("ok    " if ok else "FALLO ") + rel)
        failed += not ok
    return failed == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="solo comprueba que lo generado está al día")
    parser.add_argument("--verify", action="store_true", help="comprueba las fuentes contra SOURCES.json")
    args = parser.parse_args()
    if args.verify:
        sys.exit(0 if verify() else 1)
    files = outputs()
    stale = stale_files(files)
    if args.check:
        bad = [p for p, text in files.items() if not p.exists() or p.read_text(encoding="utf-8") != text]
        for p in bad + stale:
            print(f"gen-themes: desactualizado: {p.relative_to(ROOT)}", file=sys.stderr)
        if bad or stale:
            print("gen-themes: ejecuta scripts/gen-themes.py", file=sys.stderr)
            sys.exit(1)
        print(f"gen-themes: {len(files)} archivos al día")
        return
    for p in stale:
        p.unlink()
    for p, text in files.items():
        p.parent.mkdir(parents=True, exist_ok=True)
        if not p.exists() or p.read_text(encoding="utf-8") != text:
            p.write_text(text, encoding="utf-8")
    print(f"gen-themes: {len(files)} archivos generados")


if __name__ == "__main__":
    main()
