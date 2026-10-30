#!/usr/bin/env python3
"""Render DonationCard.svelte into a standalone HTML page for review.

Reuses the component's real <style> block and the theme variables from
themes.ts, so what you see matches the app. Writes target/donation-preview.html.

Character symbols are emitted as literal codepoints, not HTML entities —
`&jupiter;` is not a valid named entity and renders as text.
"""
import base64, pathlib, re

ROOT = pathlib.Path(__file__).resolve().parent.parent
JUPITER = "\u2643"  # ♃

style = re.search(r'<style>(.*?)</style>',
                  (ROOT / 'src/lib/DonationCard.svelte').read_text(), re.S).group(1)
themes_ts = (ROOT / 'src/lib/themes.ts').read_text()
logo = base64.b64encode((ROOT / 'static/logo.png').read_bytes()).decode()

VAR = {'bgBase': '--bg-base', 'bgSurface': '--bg-surface', 'bgElevated': '--bg-elevated',
       'bgHighlight': '--bg-highlight', 'textPrimary': '--text-primary',
       'textSecondary': '--text-secondary', 'textMuted': '--text-muted',
       'accent': '--accent', 'accentSoft': '--accent-soft', 'danger': '--danger'}


def theme(key):
    block = re.search(rf'\n  {key}:\s*\{{(.*?)\n  \}},', themes_ts, re.S).group(1)
    pairs = dict(re.findall(r"(\w+):\s*'([^']+)'", block))
    return "\n".join(f"      {VAR[k]}: {v};" for k, v in pairs.items() if k in VAR)


def card(birthday):
    if birthday:
        h2 = "Happy birthday to the creator of AstroHour!"
        ask = ("If it's earned a place in your day, please consider a $10 donation "
               "to celebrate her joyous solar return.")
    else:
        h2 = "Thank you for using AstroHour."
        ask = ("If it's earned a place in your day, a $10 donation keeps it free"
               "\u2026 and is perfect for Jupiter hour.")
    return f'''
  <div class="scrim">
    <div class="card" role="dialog" tabindex="-1">
      <img class="logo" src="data:image/png;base64,{logo}" alt="AstroHour" />
      <h2>{h2}</h2>
      <p class="lede">Ad-free, no paywalls, no accounts.</p>
      <p class="ask">{ask}</p>
      <button class="donate"><span class="jupiter">{JUPITER}</span> Donate</button>
      <div class="dismiss"><button>Not now</button><button>Don't ask again</button></div>
    </div>
  </div>'''


def panel(key, label, birthday):
    return f'''
  <div class="col">
    <h3>{label}</h3>
    <div class="stage" style="{theme(key)}">{card(birthday)}</div>
  </div>'''


html = f'''<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>
  :root {{ color-scheme: dark; }}
  body {{ margin:0; padding:20px; background:#0b0b10; color:#e8ecf5;
    font:14px/1.5 -apple-system,"Segoe UI",system-ui,sans-serif;
    display:flex; gap:18px; flex-wrap:wrap; align-items:flex-start; }}
  .col > h3 {{ font-size:11px; letter-spacing:.08em; text-transform:uppercase;
    color:#6b7280; margin:0 0 8px 2px; font-weight:700; }}
  .stage {{ position:relative; width:340px; height:430px; border-radius:10px;
    overflow:hidden; background: var(--bg-base); }}
  /* the component positions the scrim fixed; scope it to the stage instead */
  .stage .scrim {{ position:absolute; }}
{style}
</style></head><body>
{panel('midnight','Midnight — usual', False)}
{panel('midnight','Midnight — 11 Oct', True)}
{panel('lunar','Lunar', False)}
{panel('parchment','Parchment', False)}
</body></html>'''

(ROOT / 'target/donation-preview.html').write_text(html)
print("wrote target/donation-preview.html")
