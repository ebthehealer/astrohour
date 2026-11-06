# astrohour.app

A plain static site. No framework, no dependencies beyond Python 3.

| File | What to change |
|---|---|
| `assets/site.css` | Colours, fonts, spacing: the design tokens are at the top |
| `layout.html` | Header, footer and navigation shared by every page |
| `pages/*.html` | The content of each page |
| `config.json` | Stripe links, contact email, GitHub links |
| `assets/` | Screenshots, icon, favicon |

## Build and preview

```bash
python3 build.py                          # writes public/
busybox httpd -f -p 8300 -h public        # or: python3 -m http.server 8300 -d public
```

`public/` is committed, so any static host (GitHub Pages, Cloudflare Pages,
Netlify) can publish it as-is, with no build step.

## Turning on donations

1. In the Stripe dashboard create **Payment Links** (one-time): fixed prices of
   $5, $10, $20 and $30, and one product with **"Let customers choose what to pay"**
   switched on for the "Other amount" button.
2. Paste each URL into `config.json` under `stripe_links`.
3. `python3 build.py`, commit and publish. Buttons with an empty link render as
   "coming soon", and the notice disappears once any link is set.

## Screenshots

`assets/shot-*.png` were rendered from the real built app with sample data
(Boston, 30 Oct 2026). Replace them with real captures whenever you like and
keep the file names.
