# AstroHour

Planetary hours, moon phase, and lunar day — in your status bar.

Free download. $10 one-time unlock for premium features (macOS/Windows).  
Free on Linux and Noctalia with optional donation.

## Repository structure

```
astrohour/
├── src/              SvelteKit frontend (popup UI)
├── src-tauri/        Tauri v2 Rust backend
├── noctalia/         Noctalia v5 Luau plugin
└── worker/           Cloudflare Worker (Stripe → license key → email)
```

`astrohour-core` (pure Rust calculation library) lives in a separate repo.

## Platforms

| Platform | Status |
|---|---|
| macOS | In development |
| Windows | In development |
| Linux | In development — all features free |
| Noctalia | In development — all features free |
| Android | Planned |
| iOS | Planned |

## Development

### Prerequisites
- Rust (stable) + Cargo
- Node.js 18+ + npm
- Tauri CLI v2: `cargo install tauri-cli`
- `astrohour-core` checked out at `../astrohour-core`

### Desktop app
```bash
npm install
npm run tauri dev
```

### Linux: taskbar / dock icon
On Wayland the compositor ignores the icon embedded in the window. It resolves
the taskbar icon by matching the window's **app_id** (`com.astrohour.app`, set
via `enableGTKAppId` in `tauri.conf.json`) against an installed `.desktop` file.
Without one you get a generic placeholder — note this is separate from the
system-tray icon, which is drawn directly over DBus and always works.

Install the desktop entry and hicolor icons into your local XDG dirs:
```bash
./scripts/install-dev-desktop.sh
```
Re-run it whenever the icon artwork changes. Takes effect on next launch.

(Proper `.deb`/`.rpm`/`.AppImage` packaging will install these system-wide; the
script is for dev machines.)

### Icons
Source artwork lives in `assets/`. To regenerate every platform icon set after
changing it:
```bash
cargo tauri icon assets/icon-source-1024.png
```
This writes macOS `.icns`, Windows `.ico` + Store logos, Linux PNGs, Android
mipmaps and iOS AppIcons into `src-tauri/icons/`.

### Noctalia plugin
See [noctalia/README.md](noctalia/README.md) for setup and local testing instructions.

### Cloudflare Worker
See [worker/DEPLOY.md](worker/DEPLOY.md) for deployment instructions.

## License

MIT
