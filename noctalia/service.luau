--!strict
-- service.luau — background daemon for the AstroHour Noctalia plugin.
--
-- Runs astrohour-cli every 30 seconds and publishes the parsed JSON
-- to noctalia.state so widget.luau and panel.luau can read it.
--
-- State key: "data"   → the full CliOutput table, or nil on first boot
--            "error"  → string error message, or nil if healthy
--            "ts"     → unix timestamp of last successful update (number)

local INTERVAL_MS = 30_000

local function refresh()
    local lat    = tostring(noctalia.getConfig("lat"))
    local lon    = tostring(noctalia.getConfig("lon"))
    local binary = tostring(noctalia.getConfig("binary") or "astrohour-cli")

    -- Spawn the CLI and capture stdout
    local ok, stdout, stderr = noctalia.exec(binary, { "--lat", lat, "--lon", lon })

    if not ok or not stdout or stdout == "" then
        local msg = stderr or "astrohour-cli exited with no output"
        noctalia.setState("error", msg)
        return
    end

    local data, err = noctalia.jsonDecode(stdout)
    if err or data == nil then
        noctalia.setState("error", "JSON parse failed: " .. tostring(err))
        return
    end

    noctalia.setState("data",  data)
    noctalia.setState("error", nil)
    noctalia.setState("ts",    os.time())
end

-- Entry point: called by Noctalia on an interval
function onUpdate()
    refresh()
end

-- Also refresh immediately on load
refresh()
noctalia.setUpdateInterval(INTERVAL_MS)
