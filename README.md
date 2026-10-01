# KeyRC

A small always-on-top keystroke display built with Tauri and Svelte.

## Configuration

KeyRC reads `~/.config/keyrc/config.toml`. Mode and color changes apply live
within about 250 ms. The file has one mode and five colors:

```toml
mode = "keys_only"

[colors]
active_bg = "#121c29"
active_fg = "#719cd6"
key_text = "#c0c8d5"
background = "#192330"
border = "#252f3c"
```

ThemeSync writes the five colors directly into this file when the desktop theme
changes, preserving `mode` and other settings. There is no separate KeyRC theme
file or palette import. Without ThemeSync, edit these colors yourself. A minimal
example is in [config.example.toml](config.example.toml).

All pressed modifiers share `active_bg` and `active_fg`. Inactive symbols use
`key_text` at 35% opacity. `background` fills the key row and inactive modifier
cells; `border` outlines them. Colors accept `#RRGGBB` or `#RRGGBBAA`; invalid or
missing colors use black/white defaults.

`mode = "full"` (the default) shows the key history and modifier row at 290 × 114.
`"keys_only"` shows only the key history at 290 × 70 with all four corners rounded.
Inline modifier symbols remain visible in both modes. Shift combinations keep
the letter label lowercase; Caps Lock makes letters uppercase.

Missing or malformed TOML uses the default colors and `full` mode. Unknown mode
values use `full`. KeyRC reads only its own configuration file.

## Window position

Position is runtime state stored in `$XDG_CACHE_HOME/keyrc/position` (normally
`~/.cache/keyrc/position`), outside the configuration directory. Coordinates
are physical pixels, including negative positions on multi-monitor desktops.
Moves and normal window closure save the position atomically.

The native window is created hidden, its size and cached position are restored,
and its initial theme is supplied to the frontend before the window is shown.
This avoids displaying the window at the default position before moving it.
The popup does not accept focus and cannot be closed through the window manager;
dragging is its only direct interaction. Use the tray icon to toggle display
mode or quit KeyRC. A missing or invalid cache uses the window manager's default
placement.

## Development

Build the standalone release binary with `bun run tauri build --no-bundle`.
The output is `src-tauri/target/release/keyrc`. Tauri embeds the built frontend
using the `custom-protocol` feature; a plain `cargo build --release` still uses
the development URL. For a direct Cargo build, build the frontend first and use
`cargo build --manifest-path src-tauri/Cargo.toml --release --features custom-protocol`.

### Recommended IDE setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

- [ ] fix func key, and some key about volume brightness
- [ ] make settings allow more custom
- [ ] make icon tray
- [ ] change name color template for use easy config
- [ ] optimize perfomance and usage of app
