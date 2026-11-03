#!/usr/bin/env python3
"""vkTERM — comprobación del LSP (Fase 2, H0): sesiones reales de vkTERM
(Zellij y Micro en un PTY) con los servidores de `bin/vkterm-plugins install`.

    scripts/lsp-check.py [ts python rust swift go]     sin argumentos: todos

Para cada lenguaje crea un proyecto temporal (un símbolo definido en un archivo
y usado en otro, un error de tipos y código mal formateado) y comprueba:
arranque del servidor, errores en el margen, información (hover), ir a la
definición, Tab, formato y que los servidores terminan al cerrar. En
TypeScript, además, referencias, .tsx y escribir en un archivo de 3.000 líneas.

Usa el LSP instalado en config/local (o VKTERM_LSP_STATE). Go se comprueba si
está instalado ahí o en VKTERM_LSP_GO_STATE. Tarda varios minutos: Rust y Swift
indexan el proyecto.
"""
import importlib.util, os, re, shutil, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("smoke", ROOT / "scripts/smoke-test.py")
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
REAL_STATE = Path(os.environ.get("VKTERM_LSP_STATE", ROOT / "config" / "local"))
results = []


def check(name, ok, detail=""):
    results.append((name, bool(ok)))
    print("  %s %s%s" % ("✔" if ok else "✘", name, ("  — " + str(detail)) if detail else ""), flush=True)
    return ok


def write(root, files):
    for rel, text in files.items():
        p = Path(root) / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)


def where(path, needle, offset=0):
    """'línea:columna' (1-based) de la primera aparición de needle."""
    for i, line in enumerate(Path(path).read_text().splitlines(), 1):
        if needle in line:
            return "%d:%d" % (i, line.index(needle) + 1 + offset)
    raise ValueError(needle)


class Lsp:
    def __init__(self, project, open_file, state=REAL_STATE):
        self.tmp = Path(tempfile.mkdtemp(prefix="vkterm-h0-"))
        st = self.tmp / "state"
        (st / "micro").mkdir(parents=True)
        os.symlink(state / "lsp", st / "lsp")
        os.symlink(state / "micro" / "plug", st / "micro" / "plug")
        self.project = Path(project)
        self.baseline = set(self._servers())
        before = m.sessions()
        self.client = m.Client([str(ROOT / "bin/vkterm"), "--here", str(project), open_file], self.tmp)
        self.name = m.wait_new_session(before, prefix="vk-", timeout=60)
        self.s = m.Session(self.name)
        m.grant_plugin(self.s)
        m.wait_until(lambda: m.micro_status(self.s), timeout=15)

    def screen(self):
        return self.s.screen("Editor")

    def status(self):
        return m.micro_status(self.s)

    def cmd(self, text, pause=0.6):
        self.client.send(m.ctrl("e"), pause=0.3)
        self.client.send(text.encode(), m.ENTER, pause=pause)

    def keys(self, *keys, pause=0.3):
        self.client.send(*keys, pause=pause)

    def wait(self, fn, timeout=30, interval=0.5):
        return m.wait_until(fn, timeout=timeout, interval=interval)

    @staticmethod
    def _servers():
        pattern = "typescript-language-server|pyright|ruff server|rust-analyzer|sourcekit-lsp|gopls"
        out = subprocess.run(["pgrep", "-f", pattern], capture_output=True, text=True).stdout
        return [int(p) for p in out.split()]

    def servers(self):
        """Servidores de lenguaje arrancados durante esta sesión."""
        return [p for p in self._servers() if p not in self.baseline]

    def close(self):
        m.run("zellij", "kill-session", self.name)


def typescript(base):

    proj = base / "tsproj"
    write(proj, {
        "tsconfig.json": '{ "compilerOptions": { "strict": true, "jsx": "preserve", "noEmit": true } }\n',
        "src/jsx.d.ts": "declare namespace JSX { interface IntrinsicElements { [name: string]: any } }\n",
        "src/util.ts": "export function suma(a: number, b: number): number {\n  return a + b;\n}\n",
        "src/main.ts": "import { suma } from \"./util\";\n\nconst total: number = suma(1, 2);\nconst mal: string = 42;\nconsole.log(total, mal);\n",
        "src/view.tsx": "export const Vista = () => <div className=\"vista\">hola</div>;\n",
        "src/feo.ts": "const  a=1;let   b   =  2;\nexport {a,b}\n",
        "src/big.ts": "".join("export const valor%d: number = %d;\n" % (i, i) for i in range(3000)),
    })
    print("TypeScript (%s)" % proj)
    t = Lsp(proj, "src/main.ts")
    try:
        started = t.wait(lambda: t.servers(), timeout=15)
        check("el servidor de TypeScript arranca solo al abrir un .ts", started, t.servers())
        # Errores: marca en el margen y mensaje abajo con el cursor en la línea.
        marked = t.wait(lambda: ">>" in t.screen(), timeout=40)
        check("marca >> en el margen de la línea con el error", marked)
        # Micro enseña el mensaje al llegar el cursor a la línea.
        t.cmd("goto 1:1")
        t.cmd("goto " + where(proj / "src/main.ts", "mal: string"))
        t.keys(m.RIGHT, pause=0.5)   # una tecla en el editor (con goto no se enseña)
        diag = t.wait(lambda: "not assignable to type 'string'" in t.screen(), timeout=5)
        check("mensaje del error abajo con el cursor en la línea", diag, t.screen().splitlines()[-1][:90])
        # Información (hover).
        t.cmd("goto " + where(proj / "src/main.ts", "suma(1"))
        t.cmd("lsp hover", pause=1.5)
        hover = t.wait(lambda: "suma(a: number, b: number): number" in t.screen(), timeout=10)
        check("información (hover) en una división", hover)
        if hover:
            t.keys(m.ctrl("q"), pause=0.8)   # cierra la división
        # Ir a la definición (otro archivo, en otra pestaña).
        t.cmd("goto " + where(proj / "src/main.ts", "suma(1"))
        t.cmd("lsp goto-definition", pause=1.5)
        gone = t.wait(lambda: t.status().startswith("src/util.ts"), timeout=10)
        check("ir a la definición abre src/util.ts", gone, t.status()[:40])
        t.keys(b"\x1b,", pause=0.8)    # Alt-, pestaña anterior (main.ts)
        t.wait(lambda: t.status().startswith("src/main.ts"), timeout=5)
        # Referencias.
        t.cmd("goto " + where(proj / "src/main.ts", "suma(1"))
        t.cmd("lsp find-references", pause=2)
        refs = t.wait(lambda: "src/util.ts" in t.screen() and "3:23:const total" in t.screen(), timeout=10)
        check("referencias en una división (util.ts y main.ts)", refs)
        if refs:
            t.keys(m.ctrl("q"), pause=0.8)
        # Sugerencias con Tab.
        t.keys(b"\x1b[1;5F", pause=0.3)   # Ctrl-End
        t.keys(m.ENTER, b"su", pause=0.5)
        t.keys(b"\t", pause=1.5)
        last = [l for l in t.screen().splitlines() if l.strip()]
        completed = t.wait(lambda: any(re.search(r"\bsuma\b", l) and "const" not in l and "import" not in l for l in t.screen().splitlines()), timeout=8)
        check("Tab completa «su» → «suma»", completed)
        # .tsx: sin errores de JSX (parche typescriptreact).
        t.cmd("tab src/view.tsx", pause=1.5)
        t.wait(lambda: t.status().startswith("src/view.tsx"), timeout=5)
        time.sleep(5)
        check(".tsx sin errores falsos de JSX", ">>" not in t.screen(), t.screen().splitlines()[-1][:80])
        # Formato.
        t.cmd("tab src/feo.ts", pause=1.5)
        t.wait(lambda: t.status().startswith("src/feo.ts"), timeout=5)
        time.sleep(2)
        t.cmd("lsp format", pause=2)
        t.keys(b"\x1bOQ", pause=1.5)    # F2: guardar
        text = (proj / "src/feo.ts").read_text()
        check("formato (TypeScript)", "const a = 1;" in text and "let b = 2;" in text, repr(text[:60]))
        # Rendimiento: escribir en un archivo de 3.000 líneas.
        t.cmd("tab src/big.ts", pause=2)
        t.wait(lambda: t.status().startswith("src/big.ts"), timeout=5)
        time.sleep(4)
        word = "velocidadDeEscritura" * 3
        start = time.time()
        for ch in word:
            t.client.send(ch.encode(), pause=0.02)
        shown = t.wait(lambda: word in t.screen(), timeout=10, interval=0.1)
        check("escribir 60 caracteres en un .ts de 3.000 líneas sin retraso", shown,
              "%.1f s hasta verlos (60 × 20 ms = 1.2 s de tecleo)" % (time.time() - start))
        servers = t.servers()
    finally:
        t.close()
    gone = m.wait_until(lambda: not t.servers(), timeout=10)
    check("los servidores terminan al cerrar la sesión", gone, t.servers())



GO_STATE = Path(os.environ.get("VKTERM_LSP_GO_STATE", REAL_STATE))

def scenario(lang, proj, main, symbol, hover, definition, error_line, error_text, feo, formatted,
             complete=None, state=REAL_STATE, timeout=40, extra=None, save_first=False):
    print("\n%s (%s)" % (lang, proj), flush=True)
    t = Lsp(proj, main, state=state)
    try:
        check("arranca el servidor", t.wait(lambda: t.servers(), timeout=20), t.servers())
        if save_first:   # rustc (cargo check) analiza al guardar, como en VS Code
            t.keys(b"\x1bOQ", pause=1.0)
        marked = t.wait(lambda: ">>" in t.screen(), timeout=timeout)
        check("marca de error en el margen", marked)
        t.cmd("goto 1:1"); t.cmd("goto " + where(proj / main, error_line)); t.keys(m.RIGHT, pause=0.5)
        check("mensaje del error", t.wait(lambda: error_text in t.screen(), timeout=5), t.screen().splitlines()[-1][:100])
        t.cmd("goto " + where(proj / main, symbol))
        t.cmd("lsp hover", pause=1.5)
        ok = t.wait(lambda: hover in t.screen(), timeout=timeout)
        check("información (hover)", ok, "" if ok else [l.strip()[:60] for l in t.screen().splitlines() if l.strip()][-3:])
        if ok:
            t.keys(m.ctrl("q"), pause=0.8)
        t.cmd("goto " + where(proj / main, symbol))
        t.cmd("lsp goto-definition", pause=1.5)
        ok = t.wait(lambda: t.status().startswith(definition), timeout=timeout)
        check("ir a la definición (otro archivo)", ok, t.status()[:45])
        if ok:
            t.keys(b"\x1b,", pause=0.8)
        if complete:
            prefix, word = complete
            t.keys(b"\x1b[1;5F", pause=0.3)
            t.keys(m.ENTER, prefix.encode(), pause=0.8)
            t.keys(b"\t", pause=1.5)
            check("Tab completa «%s» → «%s»" % (prefix, word),
                  t.wait(lambda: any(l.strip().endswith(word) for l in t.screen().splitlines()), timeout=8))
        t.cmd("tab " + feo, pause=1.5)
        t.wait(lambda: t.status().startswith(feo), timeout=5)
        time.sleep(3)
        t.cmd("lsp format", pause=3)
        t.keys(b"\x1bOQ", pause=1.5)
        text = (proj / feo).read_text()
        check("formato", text == formatted, repr(text))
        if extra:
            extra(t)
    finally:
        t.close()
    check("los servidores terminan al cerrar", m.wait_until(lambda: not t.servers(), timeout=15), t.servers())

base = Path(tempfile.mkdtemp(prefix="vkterm-lsp-check-"))
which = sys.argv[1:] or ["ts", "python", "rust", "swift", "go"]

if "ts" in which:
    typescript(base)

if "python" in which:
    p = base / "pyproj"
    write(p, {"util.py": "def suma(a: int, b: int) -> int:\n    return a + b\n",
              "main.py": "from util import suma\n\ntotal: int = suma(1, 2)\nmal: str = 42\nprint(total, mal)\n",
              "feo.py": "x=1;y  =  2\nprint( x,y )\n"})
    scenario("Python", p, "main.py", "suma(1", "def suma(", "util.py",
             "mal: str", "is not assignable", "feo.py", "x = 1\ny = 2\nprint(x, y)\n", complete=("su", "suma"))

if "rust" in which:
    p = base / "rsproj"
    crate = p / "plugins" / "vkterm"
    src = ROOT / "plugins" / "vkterm"
    crate.mkdir(parents=True)
    for item in ("Cargo.toml", "Cargo.lock", "src", ".cargo"):
        (shutil.copytree if (src / item).is_dir() else shutil.copy)(src / item, crate / item)
    lib = crate / "src/lib.rs"
    lib.write_text(lib.read_text() + "pub mod roto;\npub mod feo;\n")
    write(crate, {"src/roto.rs": "pub fn roto() -> u32 {\n    let _k = crate::commands::editor_key(\"save\");\n    let x: u32 = \"texto\";\n    x\n}\n",
                  "src/feo.rs": "pub fn feo( )->u32{1}\n"})
    scenario("Rust", p, "plugins/vkterm/src/roto.rs", "editor_key(", "fn editor_key", "plugins/vkterm/src/commands.rs",
             "let x: u32", "mismatched types", "plugins/vkterm/src/feo.rs", "pub fn feo() -> u32 {\n    1\n}\n",
             timeout=180, save_first=True)

if "swift" in which:
    p = base / "swproj"
    write(p, {"Package.swift": "// swift-tools-version:5.9\nimport PackageDescription\n\nlet package = Package(\n    name: \"App\",\n    targets: [.executableTarget(name: \"App\", path: \"Sources/App\")]\n)\n",
              "Sources/App/Util.swift": "func suma(_ a: Int, _ b: Int) -> Int {\n    return a + b\n}\n",
              "Sources/App/main.swift": "let total = suma(1, 2)\nlet mal: String = 42\nprint(total, mal)\n",
              "Sources/App/Feo.swift": "let  a=1\nlet b   =  2\nfunc feo( )->Int{return a+b}\n"})
    scenario("Swift", p, "Sources/App/main.swift", "suma(1", "func suma(_ a: Int, _ b: Int) -> Int",
             "Sources/App/Util.swift", "let mal", "Cannot convert value", "Sources/App/Feo.swift",
             "let a = 1\nlet b = 2\nfunc feo() -> Int { return a + b }\n", timeout=180)

if "go" in which and (GO_STATE / "lsp" / "bin" / "gopls").exists():
    p = base / "goproj"
    write(p, {"go.mod": "module ejemplo\n\ngo 1.27\n",
              "util.go": "package main\n\nfunc suma(a, b int) int {\n\treturn a + b\n}\n",
              "main.go": "package main\n\nimport \"fmt\"\n\nfunc main() {\n\ttotal := suma(1, 2)\n\tvar mal string = 42\n\tfmt.Println(total, mal)\n}\n",
              "feo.go": "package main\nfunc feo( )int{return 1}\n"})
    scenario("Go", p, "main.go", "suma(1", "func suma(a int, b int) int", "util.go",
             "var mal", "cannot use 42", "feo.go", "package main\n\nfunc feo() int { return 1 }\n",
             complete=("su", "suma"), state=GO_STATE, timeout=90)

failed = [r for r in results if not r[1]]
print("\n%d/%d comprobaciones correctas" % (len(results) - len(failed), len(results)))
sys.exit(1 if failed else 0)
