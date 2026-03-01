# 🦀 ratatui-showcase

An interactive terminal UI (TUI) showcasing [ratatui](https://ratatui.rs)'s widgets, layouts, and patterns.

Navigate between 5 tabs to explore different widget types — all animated and interactive.

![Rust](https://img.shields.io/badge/Rust-000?logo=rust&logoColor=white)

## Tabs

| Tab | Widgets | What it demonstrates |
|-----|---------|---------------------|
| 📝 Todo | List, Gauge, Popup | Stateful list, add/delete/toggle, text input overlay, progress gauge |
| 📊 Table | Table | Styled headers, colored cells, row selection with TableState |
| 🔋 Gauges | Gauge, LineGauge, Sparkline | Animated CPU, memory/disk bars, download progress, live sparkline |
| 📈 Charts | Chart, Sparkline | Animated sine wave (braille markers), dual sparklines |
| 📖 About | Paragraph | Styled text, summary of all features |

## Controls

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch tabs |
| `↑`/`↓` or `j`/`k` | Navigate lists and tables |
| `Enter` / `Space` | Toggle todo item |
| `a` | Add new todo (opens input popup) |
| `d` / `Delete` | Delete selected todo |
| `+` / `-` | Adjust gauge values |
| `r` | Restart download animation |
| `?` | Toggle help popup |
| `q` / `Esc` | Quit |

## Patterns Demonstrated

- **Event loop** — tick-based animation + keyboard handling with crossterm
- **StatefulWidget** — `ListState` and `TableState` for tracking selection
- **Layout system** — horizontal/vertical splits with `Constraint` (percentage, length, min)
- **Popup overlays** — centered rect calculation + `Clear` widget
- **Text input** — raw mode character-by-character input handling
- **Terminal setup/restore** — raw mode, alternate screen, mouse capture
- **Styling** — colors, modifiers (bold, underline, crossed-out), threshold-based coloring
- **Global vs local keybindings** — tab key always switches, other keys are context-dependent

## Build & Run

```bash
cargo run

# or release build (smoother animations)
cargo run --release
```

## Dependencies

Only two:

```toml
ratatui = "0.29"    # TUI framework (widgets, layout, rendering)
crossterm = "0.28"  # Terminal backend (raw mode, events, cursor)
```

## See Also

- [clap-showcase](https://github.com/turtleeverywhere/clap-showcase) — companion project for non-interactive CLI tools with clap
