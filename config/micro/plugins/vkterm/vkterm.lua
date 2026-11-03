-- vkTERM — plugin de Micro: el tema de color, envolver la selección, el menú
-- contextual y la memoria de las pestañas del editor. bin/vkterm-workspace lo
-- copia a local/micro/plug/vkterm.
--
-- Tema (en todos los Micro de la sesión, también el de `git commit` o el
-- $EDITOR de los agentes): cada 400 ms mira el tema guardado (local/theme,
-- que escribe bin/vkterm-theme al elegirlo con ⌘K ⌘T) y, si cambió, pasa al
-- esquema vkterm-<id>. Así ningún script escribe teclas en el editor.
--
-- Envolver la selección (como editor.autoSurround de VS Code): con texto
-- seleccionado, ( [ { " ' ` lo envuelven en vez de sustituirlo, en un solo
-- paso de deshacer y con la selección sobre el texto de dentro.
--
-- Menú contextual (clic derecho, MouseRight en bindings.json), como en VS
-- Code: fuera de la selección, el cursor va al sitio del clic; después se
-- avisa al explorador de vkTERM (`zellij pipe`, mensaje context_menu), que
-- abre el menú ahí (plugins/vkterm/src/contextmenu.rs) y ejecuta lo elegido
-- en este panel con las teclas de Micro. Fuera de Zellij no hay menú.
--
-- Memoria: solo en el Micro del panel Editor (bin/vkterm-editor exporta
-- VKTERM_EDITOR_PANE=1); los demás comparten la configuración y no deben
-- pisarla. Cada vez que cambian las pestañas, la activa o lo que está sin guardar,
-- escribe projects/<huella>/editor:
--   tab=<ruta absoluta>        las pestañas, en orden
--   active=<ruta absoluta>     la pestaña activa
--   modified=<ruta absoluta>   archivos sin guardar
--   unnamed=<n>                pestañas sin nombre con texto
-- Así, al cerrar la carpeta o la ventana no hay que preguntarle nada a Micro,
-- que puede terminar sin aviso.
--
-- Comandos para bin/vkterm-state (los escribe bin/vkterm-micro):
--   vkterm.saveall   guarda los archivos modificados
--   vkterm.discard   descarta sus cambios y sus copias de seguridad, para
--                    que la próxima vez Micro no ofrezca recuperarlos
VERSION = "1.1.0"

local micro = import("micro")
local buffer = import("micro/buffer")
local config = import("micro/config")
local shell = import("micro/shell")
local regexp = import("regexp")
local util = import("micro/util")
local util = import("micro/util")
local goos = import("os")
local time = import("time")
local utf8 = import("utf8")

local state = goos.Getenv("VKTERM_PROJECT_STATE")
local enabled = goos.Getenv("VKTERM_EDITOR_PANE") == "1" and state ~= ""
local last = nil

-- Búferes de archivo de cada pestaña (con sus divisiones), en orden.
local function each_buffer(fn)
    local tabs = micro.Tabs().List
    for i = 1, #tabs do
        local panes = tabs[i].Panes
        for j = 1, #panes do
            local ok, buf = pcall(function() return panes[j].Buf end)
            if ok and buf ~= nil and not buf.Type.Scratch then
                fn(i, buf)
            end
        end
    end
end

local function snapshot()
    local lines, modified, seen, unnamed = {}, {}, {}, 0
    each_buffer(function(_, buf)
        if buf.Path == "" then
            if buf:Modified() then unnamed = unnamed + 1 end
        elseif not seen[buf.AbsPath] then
            seen[buf.AbsPath] = true
            table.insert(lines, "tab=" .. buf.AbsPath)
            if buf:Modified() then table.insert(modified, "modified=" .. buf.AbsPath) end
        end
    end)
    local tabs = micro.Tabs()
    local current = tabs.List[tabs:Active() + 1]:CurPane()
    if current ~= nil and current.Buf.Path ~= "" then
        table.insert(lines, "active=" .. current.Buf.AbsPath)
    end
    for _, line in ipairs(modified) do table.insert(lines, line) end
    if unnamed > 0 then table.insert(lines, "unnamed=" .. unnamed) end
    return table.concat(lines, "\n") .. "\n"
end

local function remember()
    if not enabled then return end
    local ok, text = pcall(snapshot)
    if not ok or text == last then return end
    local path = state .. "/editor"
    local file = io.open(path .. ".tmp", "w")
    if file == nil then return end
    file:write(text)
    file:close()
    if os.rename(path .. ".tmp", path) then last = text end
end

function onAnyEvent()
    remember()
end

-- ------------------------------------------------------------------- tema

local theme_file = config.ConfigDir .. "/../theme"
local THEME_POLL = time.ParseDuration("400ms")

local function exists(path)
    local file = io.open(path, "r")
    if file == nil then return false end
    file:close()
    return true
end

local function watch_theme()
    local file = io.open(theme_file, "r")
    if file ~= nil then
        local id = nil
        for line in file:lines() do
            id = line:match("^id=(.+)$") or id
        end
        file:close()
        local scheme = id and ("vkterm-" .. id)
        if scheme and scheme ~= config.GetGlobalOption("colorscheme")
            and exists(config.ConfigDir .. "/colorschemes/" .. scheme .. ".micro") then
            config.SetGlobalOptionNative("colorscheme", scheme)
        end
    end
    micro.After(THEME_POLL, watch_theme)
end

-- ------------------------------------------------------ envolver la selección

local SURROUND = { ["("] = ")", ["["] = "]", ["{"] = "}", ['"'] = '"', ["'"] = "'", ["`"] = "`" }

-- Micro llama a preRune una vez por cursor, con bp.Cursor en ese cursor.
function preRune(bp, r)
    local close = SURROUND[r]
    local c = bp.Cursor
    if close == nil or not c:HasSelection() then return true end
    local first, last = -c.CurSelection[1], -c.CurSelection[2]
    if last:LessThan(first) then first, last = last, first end
    local text = util.String(c:GetSelection())
    bp.Buf:Replace(first, last, r .. text .. close)
    local inner = first:Move(1, bp.Buf)
    local finish = inner:Move(utf8.RuneCountInString(text), bp.Buf)
    c:SetSelectionStart(inner)
    c:SetSelectionEnd(finish)
    c:GotoLoc(finish)
    return false
end

-- ------------------------------------------------------- menú contextual

local zellij_pane = goos.Getenv("ZELLIJ_PANE_ID")

-- Como VS Code: el clic derecho dentro de una selección la conserva.
local function in_selection(bp, loc)
    local cursors = bp.Buf:GetCursors()
    for i = 1, #cursors do
        local c = cursors[i]
        if c:HasSelection() then
            local first, last = -c.CurSelection[1], -c.CurSelection[2]
            if last:LessThan(first) then first, last = last, first end
            if first:LessEqual(loc) and loc:LessEqual(last) then return true end
        end
    end
    return false
end

function contextmenu(bp, e)
    if zellij_pane == "" then return false end
    local x, y = e:Position()
    local view = bp:BufView()
    if y >= view.Y + view.Height then return false end
    local loc = bp:LocFromVisual(buffer.Loc(x, y))
    if not in_selection(bp, loc) then
        bp:RemoveAllMultiCursors()
        local c = bp.Buf:GetActiveCursor()
        c:ResetSelection()
        c:GotoLoc(loc)
        c:StoreVisualX()
        bp:Relocate()
    end
    shell.JobSpawn("zellij", { "pipe", "--name", "context_menu", "--", zellij_pane .. " " .. x .. " " .. y },
        nil, nil, function() end)
    return true
end

function postinit()
    watch_theme()
    if not enabled then return end
    -- La pestaña activa de la última vez (bin/vkterm-editor la calcula).
    local active = tonumber(goos.Getenv("VKTERM_ACTIVE_TAB"))
    local tabs = micro.Tabs()
    if active ~= nil and active >= 1 and active <= #tabs.List then
        tabs:SetActive(active - 1)
    end
    remember()
end

-- Un argumento de la línea de comandos de Micro, entre comillas.
local function quote(s)
    return '"' .. s:gsub("\\", "\\\\"):gsub('"', '\\"') .. '"'
end

function init()
    -- «Reemplazar» (⌥⌘F), como Ctrl+H de VS Code: pide qué buscar (la selección,
    -- si la hay) y con qué reemplazarlo, y pregunta si todas a la vez o una a
    -- una (y: esta, n: saltar, Esc: parar) con el «replace» de Micro. El texto es
    -- literal y no distingue mayúsculas, como la búsqueda de VS Code.
    config.MakeCommand("vkterm.replace", function(bp)
        local selected = ""
        if bp.Cursor:HasSelection() then
            selected = util.String(bp.Cursor:GetSelection())
            if selected:find("\n") then selected = "" end
        end
        micro.InfoBar():Prompt("Buscar: ", selected, "vkterm_find", nil, function(find, canceled)
            if canceled or find == "" then return end
            micro.InfoBar():Prompt("Reemplazar con: ", "", "vkterm_replace", nil, function(with, canceled2)
                if canceled2 then return end
                micro.InfoBar():YNPrompt("¿Reemplazar todas las coincidencias? (y: todas, n: una a una, Esc: cancelar)", function(all, canceled3)
                    if canceled3 then return end
                    bp:HandleCommand("replace " .. quote("(?i)" .. regexp.QuoteMeta(find)) .. " " .. quote((with:gsub("%$", "$$"))) .. (all and " -a" or ""))
                end)
            end)
        end)
    end, config.NoComplete)
    -- «Mostrar en Finder» del menú contextual: el archivo de esta pestaña.
    config.MakeCommand("vkterm.reveal", function(bp)
        local path = bp.Buf.AbsPath
        if bp.Buf.Path == "" or path == "" then
            micro.InfoBar():Error("Este archivo aún no está guardado")
            return
        end
        shell.JobSpawn("open", { "-R", path }, nil, nil, function() end)
    end, config.NoComplete)
    config.MakeCommand("vkterm.saveall", function()
        each_buffer(function(_, buf)
            if buf.Path ~= "" and buf:Modified() then buf:Save() end
        end)
        remember()
    end, config.NoComplete)
    config.MakeCommand("vkterm.discard", function()
        each_buffer(function(_, buf)
            if buf.Path ~= "" and buf:Modified() then
                buf:ReOpen()
                buf:RemoveBackup()
            end
        end)
        remember()
    end, config.NoComplete)
end
