#!/usr/bin/env python3
"""Render the day/moon header on its own, using the app's own CSS.

The header is hard to inspect in a browser: the app needs Tauri's IPC, so a
plain page load shows only the loading glyph. This pulls the real <style>
block and the real theme values out of the source and lays the section out
with sample data, so what you look at is what ships.

    python3 scripts/preview-header.py && open target/header-preview.html
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "target" / "header-preview.html"

# Sample data, taken from a real morning so the lengths are realistic.
VALUES = {
    "dayDate": "October 30, 2026",
    "dayName": "Friday",
    "dayRuler": "Venus",
    "sunrise": "07:15 AM",
    "sunset": "05:42 PM",
    "nextNewMoon": "Nov 9 &middot; 02:01 AM",
    "nextFullMoon": "Nov 24 &middot; 09:56 AM",
    "phaseEmoji": "\U0001f317",
    "phaseName": "Waning Gibbous",
    "lunarDay": "20",
    "illumination": "79",
}


def theme_tokens() -> dict:
    """The first theme in themes.ts, as CSS custom properties."""
    src = (ROOT / "src/lib/themes.ts").read_text()
    block = src[src.index("midnight: {"):]
    block = block[: block.index("\n  },")]
    tokens = {}
    for key, value in re.findall(r"(\w+):\s*'(#[0-9a-fA-F]{3,8})'", block):
        tokens[key] = value
    return tokens


def style_block() -> str:
    src = (ROOT / "src/routes/+page.svelte").read_text()
    return re.search(r"<style>(.*?)</style>", src, re.S).group(1)


def section() -> str:
    """The .day-moon markup, with the Svelte bindings substituted."""
    src = (ROOT / "src/routes/+page.svelte").read_text()
    start = src.index('<section class="day-moon">')
    end = src.index("</section>", start) + len("</section>")

    html = src[start:end]
    html = re.sub(r"\{#if (\w+)\}(.*?)\{/if\}", lambda m: m.group(2) if VALUES.get(m.group(1)) else "", html, flags=re.S)
    html = re.sub(r"\{planetSymbol\((\w+)\)\}", "\u2640", html)          # Venus, as in the sample
    html = re.sub(r"\{phaseEmoji\([^)]*\)\}", VALUES["phaseEmoji"], html)
    html = re.sub(r"\{phaseName\([^)]*\)\}", VALUES["phaseName"], html)
    html = re.sub(r"\{Math\.round\(moon\.illumination \* 100\)\}", VALUES["illumination"], html)
    html = re.sub(r"\{moon\.lunar_day\}", VALUES["lunarDay"], html)
    for key in (
        "dayDate", "dayName", "dayRuler", "sunrise", "sunset",
        "nextNewMoon", "nextFullMoon",
    ):
        html = html.replace("{%s}" % key, VALUES[key])
    html = re.sub(r"\{[^}]*\}", "?", html)                                # anything left over
    return html.strip()


def main() -> int:
    tokens = theme_tokens()
    if not tokens:
        print("could not read any theme values from themes.ts", file=sys.stderr)
        return 1

    # themes.ts names its keys in camelCase; the stylesheet reads kebab-case.
    def kebab(name: str) -> str:
        return re.sub(r"(?<!^)(?=[A-Z])", "-", name).lower()

    variables = "\n".join(f"      --{kebab(k)}: {v};" for k, v in tokens.items())

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(f"""<!doctype html>
<html><head><meta charset="utf-8"><title>AstroHour header</title>
<style>
  :root {{
{variables}
  }}
  body {{
    margin: 0;
    background: var(--bgBase);
    font-family: system-ui, sans-serif;
    color: var(--textPrimary);
    width: 420px;
  }}
{style_block()}
</style></head>
<body>
{section()}
</body></html>
""")
    print(f"wrote {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
