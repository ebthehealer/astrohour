# AstroHour for Noctalia

Planetary hours, moon phase, and lunar day — in your Noctalia status bar.

## Features

- **Bar widget** — current planetary hour symbol + name, updates every 30 seconds
- **Panel** — click the bar widget to see the full 24-hour schedule, moon phase, lunar day, sunrise/sunset
- **Three display modes** — icon only (`☉`), short (`☉ Sun Hour`), or full (`☉ Sun Hour (Night) · 23m`)
- **Planet colour coding** — subtle tint per planet using your theme colours
- **All features free** on Noctalia — optional $10 donation if you find it useful

## Requirements

- Noctalia v5 (plugin_api 19+)
- `astrohour-cli` binary on your system

## Installing astrohour-cli

Build from source (requires Rust):

```bash
git clone https://github.com/YOUR_USERNAME/astrohour-core
cd astrohour-core
cargo build -p astrohour-cli --release
# Copy to somewhere on your PATH:
cp target/release/astrohour-cli ~/.local/bin/
```

Pre-built binaries for x86_64 and aarch64 (Fedora Asahi) will be available
at https://astrohour.app/download once the project is in stable release.

## Installation

### From the plugin store (once published)
```
noctalia msg plugins enable astrohour/astrohour
```

### Local development / testing
```bash
git clone https://github.com/YOUR_USERNAME/astrohour
noctalia msg plugins source add astrohour path ~/path/to/astrohour-noctalia
noctalia msg plugins enable astrohour/astrohour
```

## Configuration

Set your coordinates in the plugin settings (Noctalia settings → Plugins → AstroHour):

| Setting | Default | Description |
|---|---|---|
| `lat` | `40.7128` | Your latitude |
| `lon` | `-74.0060` | Your longitude |
| `display` | `short` | Bar text: `icon`, `short`, or `full` |
| `binary` | `astrohour-cli` | Path to the CLI binary |

Or via config file:

```toml
[plugin.astrohour/astrohour]
lat     = 51.5074
lon     = -0.1278
display = "short"
binary  = "/home/you/.local/bin/astrohour-cli"
```

## Support

Free on Noctalia. If you find it useful, consider a $10 donation at https://astrohour.app/donate

Issues: https://github.com/YOUR_USERNAME/astrohour/issues
