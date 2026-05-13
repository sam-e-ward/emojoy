# Emojoy 😀

A lightweight desktop emoji picker. Type `::` anywhere to trigger a floating search popup, find your emoji, hit enter — it's pasted into your active app.

## Features

- **Global trigger**: Type `::` in any app to open the picker
- **Smart search**: Tag-based search with multiple aliases per emoji (powered by [emojilib](https://github.com/muan/emojilib))
- **Keyboard-first**: Arrow keys to navigate, Enter to select, Escape to dismiss
- **Learns from you**: Most-used emojis float to the top
- **Lightweight**: ~8MB app, minimal resource usage
- **Custom aliases**: Add your own search terms via config file

## Install (macOS)

### Prerequisites

- [Rust](https://rustup.rs/) — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- [Node.js](https://nodejs.org/) (v18+)

### Build & install

```bash
git clone https://github.com/samward/emojoy.git
cd emojoy
./install.sh
```

### Grant Accessibility permission

Emojoy needs Accessibility access to detect `::` keystrokes globally:

1. Open **System Settings → Privacy & Security → Accessibility**
2. Click **+** and add **Emojoy** from `/Applications`
3. Restart Emojoy

### Launch at login (optional)

**System Settings → General → Login Items** → **+** → add **Emojoy**

## Usage

1. Type `::` in any text field — the picker appears
2. Start typing to search (e.g. `fire`, `laugh`, `heart`)
3. **↑↓** to navigate, **Enter** to paste, **Escape** to dismiss
4. Click away to dismiss
5. Your most-used emojis rise to the top over time

## Configuration

Edit `~/.config/emojoy/config.json`:

```json
{
  "trigger_sequence": "::",
  "custom_aliases": {
    "🔥": ["awesome", "cool", "nice"],
    "😂": ["rofl", "dead", "dying"]
  }
}
```

Usage stats are stored at `~/.config/emojoy/usage.json`.

## Development

```bash
npm install
npm run tauri dev
```

## License

GPL-3.0 — see [LICENSE](LICENSE)
