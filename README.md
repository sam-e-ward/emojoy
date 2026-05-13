# Emojoy 😀

A lightweight desktop emoji picker. Type `::` anywhere to trigger a floating search popup, find your emoji, hit enter — it's pasted into your active app.

## Features

- **Global trigger**: Type `::` in any app to open the picker
- **Smart search**: Tag-based search with multiple aliases per emoji (powered by [emojilib](https://github.com/muan/emojilib))
- **Keyboard-first**: Arrow keys to navigate, Enter to select, Escape to dismiss
- **Learns from you**: Most-used emojis float to the top
- **Lightweight**: ~4MB download, minimal resource usage
- **Custom aliases**: Add your own search terms via config file

## Install (macOS)

### Prerequisites

- [Rust](https://rustup.rs/) — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- [Node.js](https://nodejs.org/) (v18+)

### Build & install

```bash
git clone https://github.com/samward/emojoy.git
cd emojoy
npm install
npm run tauri build
```

Then either:

- **Drag to Applications**: Open `src-tauri/target/release/bundle/dmg/Emojoy_*.dmg` and drag Emojoy to your Applications folder
- **Or copy directly**: `cp -r src-tauri/target/release/bundle/macos/Emojoy.app /Applications/`

### Grant Accessibility permission

Emojoy needs Accessibility access to detect your `::` keystrokes globally:

1. Open **System Settings → Privacy & Security → Accessibility**
2. Click the **+** button and add **Emojoy** (from Applications)
3. Restart Emojoy

> Without this permission, the `::` trigger won't work. The app will still run in the menu bar but you won't be able to activate it.

### Launch at login (optional)

1. Open **System Settings → General → Login Items**
2. Click **+** and add **Emojoy**

## Usage

1. Type `::` in any text field — the picker appears
2. Start typing to search (e.g. `fire`, `laugh`, `heart`)
3. **↑↓** to navigate, **Enter** to paste, **Escape** to dismiss
4. Click away to dismiss
5. Your most-used emojis rise to the top over time

## Configuration

Config file: `~/.config/emojoy/config.json`

```json
{
  "trigger_sequence": "::",
  "custom_aliases": {
    "🔥": ["awesome", "cool", "nice"],
    "😂": ["rofl", "dead", "dying"]
  }
}
```

- **trigger_sequence**: The key sequence that opens the picker (default `::`)
- **custom_aliases**: Extra search terms for any emoji, merged with the built-in tags

Usage stats are stored at `~/.config/emojoy/usage.json`.

## Tech Stack

- [Tauri v2](https://tauri.app/) — Rust backend, web frontend
- Vanilla HTML/CSS/JS frontend
- macOS: CoreGraphics for global keystroke monitoring and paste simulation

## License

GPL-3.0 — see [LICENSE](LICENSE)
