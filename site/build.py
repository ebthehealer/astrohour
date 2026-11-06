#!/usr/bin/env python3
"""Build the static site: pages/*.html wrapped in layout.html -> public/.

No dependencies. Run `python3 build.py`, then publish the `public/` folder
to any static host. Each page starts with `<!-- title: ... -->` and (optionally)
`<!-- desc: ... -->`.
"""
import json, re, shutil, pathlib, html

root = pathlib.Path(__file__).parent
cfg = json.loads((root / "config.json").read_text())
layout = (root / "layout.html").read_text()
out = root / "public"

if out.exists():
    shutil.rmtree(out)
shutil.copytree(root / "assets", out / "assets")

def donate_btn(key: str, label: str, cls: str = "") -> str:
    url = cfg["stripe_links"].get(key, "").strip()
    if url:
        return f'<a class="amount {cls}" href="{html.escape(url)}" rel="noopener">{label}</a>'
    return f'<span class="amount {cls} is-off" title="Donations open soon" aria-disabled="true">{label}</span>'

subs = {
    "site_url": cfg["site_url"],
    "email": cfg["contact_email"],
    "github_url": cfg["github_url"],
    "releases_url": cfg["releases_url"],
    "btn_5": donate_btn("5", "$5"),
    "btn_10": donate_btn("10", "$10"),
    "btn_20": donate_btn("20", "$20"),
    "btn_30": donate_btn("30", "$30"),
    "btn_custom": donate_btn("custom", "Other amount", "custom"),
    "donate_live": "" if any(v.strip() for v in cfg["stripe_links"].values()) else
        '<p class="notice">Donations are not switched on yet. The buttons below will go live shortly.</p>',
}

PAGES = {"index": "", "donate": "donate", "about": "about", "contact": "contact", "policies": "policies"}

for name, slug in PAGES.items():
    src = (root / "pages" / f"{name}.html").read_text()
    title = re.search(r"<!--\s*title:\s*(.*?)\s*-->", src).group(1)
    m = re.search(r"<!--\s*desc:\s*(.*?)\s*-->", src)
    desc = m.group(1) if m else "AstroHour shows the current planetary hour, moon phase and Moon sign in your status bar."
    body = re.sub(r"<!--\s*(title|desc):.*?-->\n?", "", src)
    page = layout.replace("{{content}}", body)
    page = page.replace("{{title}}", html.escape(title) if name == "index" and False else title)
    page = page.replace("{{desc}}", html.escape(desc))
    page = page.replace("{{nav_" + (name) + "}}", ' aria-current="page"')
    page = re.sub(r"\{\{nav_\w+\}\}", "", page)
    for k, v in subs.items():
        page = page.replace("{{" + k + "}}", v)
    dest = out / (slug + "/index.html" if slug else "index.html")
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(page)
    print("built", dest.relative_to(root))

(out / "CNAME").write_text("astrohour.app\n")
(out / "404.html").write_text(
    layout.replace("{{content}}", '<section class="page narrow"><h1>Page not found</h1><p><a href="/">Back to the home page</a></p></section>')
    .replace("{{title}}", "Not found").replace("{{desc}}", "Page not found")
    .replace("{{site_url}}", cfg["site_url"]).replace("{{email}}", cfg["contact_email"])
    .replace("{{github_url}}", cfg["github_url"]).replace("{{releases_url}}", cfg["releases_url"])
    .replace("{{nav_index}}", "").replace("{{nav_donate}}", "").replace("{{nav_about}}", "")
    .replace("{{nav_contact}}", "").replace("{{nav_policies}}", ""))
