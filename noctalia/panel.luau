--!strict
-- panel.luau — schedule panel for AstroHour.
--
-- Opens when the user clicks the bar widget.
-- Shows:
--   - Current planetary hour (large, prominent)
--   - Moon phase + lunar day
--   - Full 24-hour schedule (scrollable)
--   - Optional donation nudge at the bottom

local DONATION_URL = "https://astrohour.app/donate"

-- ── Helpers ───────────────────────────────────────────────────────────────────

local function formatTime(unix: number): string
    -- Format as HH:MM in local time using the shell's time format
    return noctalia.formatTime("{:%H:%M}", unix)
end

local function moonIllumPct(illum: number): string
    return string.format("%d%%", math.floor(illum * 100 + 0.5))
end

-- ── Panel layout ──────────────────────────────────────────────────────────────

function onOpen()
    local data = noctalia.getState("data")
    local err  = noctalia.getState("error")

    -- Error / loading state
    if err ~= nil and data == nil then
        panel.setContent(
            ui.column({
                ui.text("⚠ AstroHour error", { style = "titleLarge", color = "error" }),
                ui.text(tostring(err), { style = "bodyMedium", color = "onSurfaceVariant" }),
                ui.text(
                    "Check that astrohour-cli is installed and the binary path setting is correct.",
                    { style = "bodySmall", color = "outline" }
                ),
            })
        )
        return
    end

    if data == nil then
        panel.setContent(ui.text("Loading…", { style = "bodyMedium" }))
        return
    end

    -- ── Current hour card ────────────────────────────────────────────
    local cur_symbol = data.current_symbol or "?"
    local cur_planet = data.current_planet or "?"
    local cur_mins   = data.minutes_remaining or 0
    local hour_idx   = data.hour_index or 0
    local day_night  = hour_idx < 12 and "Day Hour" or "Night Hour"

    local currentCard = ui.card({
        ui.row({
            ui.text(cur_symbol, { style = "displaySmall" }),
            ui.column({
                ui.text(
                    string.format("%s · %s", cur_planet, day_night),
                    { style = "titleLarge" }
                ),
                ui.text(
                    string.format("%d min remaining", cur_mins),
                    { style = "bodyMedium", color = "onSurfaceVariant" }
                ),
            }, { expand = true, padding = { left = 12 } }),
        }, { verticalAlignment = "center" }),
    }, { padding = 16 })

    -- ── Day ruler row ────────────────────────────────────────────────
    local rulerRow = ui.row({
        ui.text(
            string.format(
                "%s Planetary day of %s  ·  Sunrise %s  ·  Sunset %s",
                data.day_ruler_symbol,
                data.day_ruler,
                formatTime(data.sunrise_unix),
                formatTime(data.sunset_unix)
            ),
            { style = "labelMedium", color = "onSurfaceVariant" }
        ),
    }, { padding = { horizontal = 16, vertical = 8 } })

    -- ── Moon card ────────────────────────────────────────────────────
    local moonCard = ui.card({
        ui.row({
            ui.text(data.moon_emoji or "?", { style = "headlineMedium" }),
            ui.column({
                ui.text(
                    string.format("%s · Lunar day %d", data.moon_phase, data.lunar_day),
                    { style = "titleMedium" }
                ),
                ui.text(
                    string.format("Illumination: %s", moonIllumPct(data.moon_illumination or 0)),
                    { style = "bodySmall", color = "onSurfaceVariant" }
                ),
            }, { expand = true, padding = { left = 12 } }),
        }, { verticalAlignment = "center" }),
    }, { padding = 12, margin = { horizontal = 0, top = 0, bottom = 8 } })

    -- ── Hour schedule list ────────────────────────────────────────────
    local hourItems = {}
    if data.hours then
        for _, h in ipairs(data.hours) do
            local isCurrent = h.is_current == true
            local timeStr   = string.format("%s – %s", formatTime(h.start_unix), formatTime(h.end_unix))
            local dnLabel   = h.is_day and "☀" or "☽"

            table.insert(hourItems,
                ui.row({
                    -- Planet symbol column
                    ui.text(h.symbol or "?", {
                        style = isCurrent and "titleMedium" or "bodyMedium",
                        color = isCurrent and "primary" or "onSurface",
                        width = 28,
                    }),
                    -- Planet name + day/night
                    ui.column({
                        ui.text(
                            string.format("%s %s", h.planet or "?", dnLabel),
                            {
                                style = isCurrent and "titleMedium" or "bodyMedium",
                                color = isCurrent and "primary" or "onSurface",
                            }
                        ),
                        ui.text(timeStr, { style = "labelSmall", color = "onSurfaceVariant" }),
                    }, { expand = true }),
                    -- Current indicator
                    isCurrent
                        and ui.text("◀ now", { style = "labelSmall", color = "primary" })
                        or  ui.spacer({ width = 48 }),
                }, {
                    padding     = { horizontal = 16, vertical = 6 },
                    background  = isCurrent and "surfaceVariant" or nil,
                    borderRadius = isCurrent and 8 or nil,
                })
            )
        end
    end

    local scheduleSection = ui.column({
        ui.text("Today's Schedule", {
            style   = "labelLarge",
            color   = "onSurfaceVariant",
            padding = { horizontal = 16, top = 12, bottom = 4 },
        }),
        table.unpack(hourItems),
    })

    -- ── Donation nudge ────────────────────────────────────────────────
    -- All features are free on Linux/Noctalia. This is an optional ask.
    local donationRow = ui.row({
        ui.column({
            ui.text("Enjoying AstroHour?", { style = "labelMedium" }),
            ui.text(
                "It's free on Noctalia. A $10 donation keeps it going.",
                { style = "bodySmall", color = "onSurfaceVariant" }
            ),
        }, { expand = true }),
        ui.button("Support ♥", {
            style   = "outlined",
            onClick = function() noctalia.openUrl(DONATION_URL) end,
        }),
    }, {
        padding     = { horizontal = 16, vertical = 12 },
        background  = "surfaceVariant",
        borderRadius = 8,
        margin      = { horizontal = 0, top = 8, bottom = 4 },
    })

    -- ── Assemble full panel ───────────────────────────────────────────
    panel.setContent(
        ui.scrollable(
            ui.column({
                currentCard,
                rulerRow,
                moonCard,
                scheduleSection,
                donationRow,
            }, { padding = { horizontal = 8, top = 8, bottom = 16 } })
        )
    )

    panel.setTitle("AstroHour")
    panel.setWidth(360)
end

-- Refresh the panel each time state updates (service fires every 30s)
function onUpdate()
    onOpen()
end
