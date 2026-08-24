# KeyRC

A small always-on-top keystroke display built with Tauri and Svelte.

## ThemeSync

KeyRC reads the Base46 desktop palette from `~/.local/share/nvim/theme`. The
running app watches that palette and applies ThemeSync changes within about
250 ms; it does not need to be restarted or explicitly reloaded.

If the palette is unavailable or malformed, KeyRC falls back to its original
black and white theme. The following palette keys are used: `is_light`, `bg`,
`fg`, `bg_dark`, `border`, `grey`, `green`, `blue`, `purple`, and `cyan`.

## Development

### Recommended IDE setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

- [ ] fix func key, and some key about volume brightness
- [ ] make settings allow more custom
- [ ] make icon tray
