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

### Noctalia plugin
See [noctalia/README.md](noctalia/README.md) for setup and local testing instructions.

### Cloudflare Worker
See [worker/DEPLOY.md](worker/DEPLOY.md) for deployment instructions.

## License

MIT
