# Emojoy 😊

A lightweight desktop emoji picker. Type `::` anywhere to trigger a floating search popup, find your emoji, hit enter — it's pasted into your active app.

## Features

- **Global trigger**: Type `::` in any app to open the picker
- **Smart search**: Tag-based search with multiple aliases per emoji (powered by [emojilib](https://github.com/muan/emojilib))
- **Keyboard-first**: Arrow keys to navigate, Enter to select, Escape to dismiss
- **Lightweight**: ~8MB app bundle, minimal resource usage
- **Custom aliases**: Add your own search terms via config file

## Tech Stack

- [Tauri v2](https://tauri.app/) — Rust backend, web frontend
- Vanilla HTML/CSS/JS frontend
- macOS: CoreGraphics event tap for global keystroke monitoring

## Development

```bash
npm install
npm run tauri dev
```

## Building

```bash
npm run tauri build
```

## Configuration

Config lives at `~/.config/emojoy/config.json`:

```json
{
  "trigger_sequence": "::",
  "custom_aliases": {
    "🔥": ["awesome", "cool"],
    "😂": ["rofl", "dead"]
  }
}
```

## Requirements

- **macOS**: Accessibility permission required (System Preferences → Privacy & Security → Accessibility → Enable Emojoy)
- **Windows**: Coming soon

## License

GPL-3.0 — see [LICENSE](LICENSE)
