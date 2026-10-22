-- vkTERM — configuración de µlsp (mlsp). bin/vkterm-plugins la copia como
-- plug/mlsp/config.lua en lugar de la del plugin.
--
-- Cada servidor es un ejecutable de $VKTERM_STATE_DIR/lsp/bin, que instala
-- bin/vkterm-plugins con rutas absolutas (el PATH de la sesión no importa).
-- Solo se arrancan los que están instalados.
local goos = import("os")
local filepath = import("path/filepath")
local strings = import("strings")
local config = import("micro/config")

local lsp = goos.Getenv("VKTERM_STATE_DIR") .. "/lsp/"

local function installed(name)
    local _, err = goos.Stat(lsp .. "bin/" .. name)
    return err == nil
end

-- rust-analyzer solo busca Cargo.toml en la raíz y un nivel por debajo. Si no
-- hay uno en la raíz, se le pasan los que haya hasta tres niveles (como
-- plugins/vkterm/Cargo.toml en vkTERM), sin target/ ni node_modules/.
local function rustProjects()
    local root, _ = goos.Getwd()
    local found = {}
    for _, pattern in ipairs({"Cargo.toml", "*/Cargo.toml", "*/*/Cargo.toml", "*/*/*/Cargo.toml"}) do
        local matches, _ = filepath.Glob(root .. "/" .. pattern)
        -- Las listas de Go no admiten # en Lua: se recorren como texto.
        for path in strings.Join(matches, "\n"):gmatch("[^\n]+") do
            if pattern == "Cargo.toml" then return nil end
            if not path:find("/target/") and not path:find("/node_modules/") then
                table.insert(found, path)
            end
        end
    end
    if #found == 0 then return nil end
    return { linkedProjects = found }
end

-- Valores por defecto de cada servidor (los de µlsp).
local defaultLanguageServerOptions = {
    shortName = nil,
    cmd = "",
    args = {},
    filetypes = nil,
    initializationOptions = nil,
    onInitialized = nil,
}

languageServer = {
    typescript = {
        shortName = "typescript",
        cmd = lsp .. "bin/typescript-language-server",
        args = {"--stdio"},
        filetypes = {"javascript", "typescript"},
        -- Siempre la TypeScript de vkTERM: la 7 (nativa) no trae tsserver.
        initializationOptions = {
            tsserver = { path = lsp .. "node/node_modules/typescript/lib" },
        },
    },
    pyright = {
        shortName = "pyright",
        cmd = lsp .. "bin/pyright-langserver",
        args = {"--stdio"},
        filetypes = {"python"},
    },
    ruff = {
        shortName = "ruff",
        cmd = lsp .. "bin/ruff",
        args = {"server"},
        filetypes = {"python"},
        onInitialized = function(client)
            -- La información de pyright es mejor. nil y no false: µlsp solo
            -- mira si la capacidad existe, y con false le seguiría
            -- preguntando a ruff (sin resultados).
            client.serverCapabilities.hoverProvider = nil
        end,
    },
    rust = {
        shortName = "rust",
        cmd = lsp .. "bin/rust-analyzer",
        filetypes = {"rust"},
        initializationOptions = rustProjects(),
    },
    swift = {
        shortName = "swift",
        cmd = lsp .. "bin/sourcekit-lsp",
        filetypes = {"swift"},
    },
    go = {
        shortName = "go",
        cmd = lsp .. "bin/gopls",
        filetypes = {"go", "gomod", "godoc"},
    },
}

defaultLanguageServerOptions.__index = defaultLanguageServerOptions
for _, server in pairs(languageServer) do
    setmetatable(server, defaultLanguageServerOptions)
end

gotoMode = {
    default     = "default",
    currentPane = "currentPane",
}

-- Servidores instalados por tipo de archivo de Micro, en orden de preferencia.
local autostart, default = {}, {}
local function use(server, executable)
    if not installed(executable) then return end
    for _, filetype in ipairs(server.filetypes) do
        autostart[filetype] = autostart[filetype] or {}
        table.insert(autostart[filetype], server)
        default[filetype] = default[filetype] or server
    end
end
use(languageServer.typescript, "typescript-language-server")
use(languageServer.pyright, "pyright-langserver")
use(languageServer.ruff, "ruff")
use(languageServer.rust, "rust-analyzer")
use(languageServer.swift, "sourcekit-lsp")
use(languageServer.go, "gopls")

-- Formatear al guardar: desactivado, como en VS Code (editor.formatOnSave).
-- Se activa con "mlsp.formatonsave": true en config/micro/settings.json.
config.RegisterGlobalOption("mlsp", "formatonsave", false)

settings = {
    -- Tab completa con el servidor de lenguaje cuando lo hay.
    tabAutocomplete = true,
    formatOnSave = config.GetGlobalOption("mlsp.formatonsave") == true,
    autostart = autostart,
    defaultLanguageServer = default,
    ignoreBuffersWithUnknownFiletype = false,
    showDiagnostics = {
        error       = true,
        warning     = true,
        information = false,
        hint        = false,
    },
    gotoMode = gotoMode.default,
}
