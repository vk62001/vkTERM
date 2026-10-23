-- vkTERM — memoria de las pestañas del editor (bin/vkterm-state).
--
-- bin/vkterm-workspace copia este plugin a local/micro/plug/vkterm. Solo
-- actúa en el Micro del panel Editor (bin/vkterm-editor exporta
-- VKTERM_EDITOR_PANE=1): el de `git commit` o el $EDITOR de los agentes
-- comparten la configuración y no deben pisar la memoria.
--
-- Cada vez que cambian las pestañas, la activa o lo que está sin guardar,
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
VERSION = "1.0.0"

local micro = import("micro")
local config = import("micro/config")
local goos = import("os")

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

function postinit()
    if not enabled then return end
    -- La pestaña activa de la última vez (bin/vkterm-editor la calcula).
    local active = tonumber(goos.Getenv("VKTERM_ACTIVE_TAB"))
    local tabs = micro.Tabs()
    if active ~= nil and active >= 1 and active <= #tabs.List then
        tabs:SetActive(active - 1)
    end
    remember()
end

function init()
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
