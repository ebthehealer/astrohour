--!strict
-- widget.luau — bar widget for AstroHour.
--
-- Shows the current planetary hour in the status bar.
-- Clicking opens the schedule panel.
--
-- Display depends on the "display" setting:
--   icon  → "☉"
--   short → "☉ Sun Hour"          (default)
--   full  → "☉ Sun Hour (Night) · 23m"

local function getDisplay(): string
    local mode = tostring(noctalia.getConfig("display") or "short")
    local data = noctalia.getState("data")
    local err  = noctalia.getState("error")

    -- Error state: show a small warning glyph
    if err ~= nil and data == nil then
        return "⚠ AstroHour"
    end

    -- Loading state (first boot, data not yet available)
    if data == nil then
        return "◌"
    end

    local symbol = data.current_symbol or "?"
    local planet = data.current_planet or "?"
    local mins   = data.minutes_remaining or 0
    local is_day = data.hour_index ~= nil and data.hour_index < 12

    if mode == "icon" then
        return symbol
    elseif mode == "full" then
        local dn = is_day and "Day" or "Night"
        return string.format("%s %s Hour (%s) · %dm", symbol, planet, dn, mins)
    else
        -- "short" is the default
        return string.format("%s %s Hour", symbol, planet)
    end
end

-- Called by Noctalia each time the widget should re-render.
-- The service refreshes state every 30s; Noctalia calls update on state change.
function update()
    barWidget.setText(getDisplay())

    -- Tint the widget text based on planet (subtle colour coding)
    local data = noctalia.getState("data")
    if data and data.current_planet then
        local planet = data.current_planet
        if     planet == "Sun"     then barWidget.setColor("accent")
        elseif planet == "Moon"    then barWidget.setColor("onSurface")
        elseif planet == "Mars"    then barWidget.setColor("error")
        elseif planet == "Mercury" then barWidget.setColor("secondary")
        elseif planet == "Jupiter" then barWidget.setColor("tertiary")
        elseif planet == "Venus"   then barWidget.setColor("primary")
        elseif planet == "Saturn"  then barWidget.setColor("outline")
        else                            barWidget.setColor("onSurface")
        end
    end
end

-- Open the schedule panel on click
function onClick()
    noctalia.openPanel("astrohour/schedule")
end
