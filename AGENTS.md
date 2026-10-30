# AGENTS.md — Developer & AI Agent Guide for `mdv`

Welcome to **`mdv`** (Markdown Viewer), a native Linux, distraction-free **Markdown Focus Reader** written in Rust using Tauri v2.

---

## 1. Project Philosophy & Core Requirements

- **Distraction-Free Focus & Responsive Width**: Clean typography with flexible layout. Adapts to full window width by default, with option to toggle to a centered reading column (<kbd>W</kbd>).
- **Single-Viewport Toggle**: Toggle between **Rendered Preview** and **Raw Source** inside the *same main window* (no side-by-side panels). Relative scroll position is preserved across toggles. Source view renders line-by-line with clean non-selectable line numbers and text wrapping (<kbd>Ctrl</kbd> + <kbd>E</kbd> or <kbd>Tab</kbd>).
- **Font Switching**: Toggle between default proportional sans-serif and monospace programming font (**JetBrains Mono**) (<kbd>F</kbd>) for unambiguous glyph discrimination (`0`/`O`, `1`/`l`/`I`).
- **Document Navigation History**: Seamlessly traverse previously viewed documents with <kbd>Alt</kbd> + <kbd>←</kbd>, <kbd>Backspace</kbd>, or mouse thumb button (button 3).
- **Live Auto-Reload**: Watches files on disk via `notify`. When the user edits and saves in any text editor (Vim, Helix, Micro, VSCode), `mdv` updates the view instantly.
- **Font Zoom Controls**: Dynamic font resizing with <kbd>Ctrl</kbd> + <kbd>+</kbd> / <kbd>-</kbd> and reset with <kbd>Ctrl</kbd> + <kbd>0</kbd>.
- **Zero Frontend Tooling Bloat**: The UI is pure, dependency-free vanilla HTML, CSS, and JavaScript. No Node.js build pipelines or `node_modules` are required to build or run.
- **Portability Across Linux Distros**: Runs on Manjaro/Arch and compiles cleanly on Debian/Ubuntu/Mint with standard `libwebkit2gtk` packages.

---

## 2. Architecture Overview

```
mdv/
├── Makefile                # Primary developer task runner (build, run, install, deps)
├── README.md               # User guide, keybindings, and desktop integration instructions
├── AGENTS.md               # Architecture and guidelines for human & AI contributors
├── SAMPLE.md               # Sample demonstration markdown document
├── .gitignore              # Rust targets, editor and OS artifacts
├── ui/                     # Pure static frontend assets (embedded into binary)
│   ├── index.html          # Semantic layout, toolbar, viewport, shortcuts modal
│   ├── style.css           # Focus typography, CSS theme variables (auto/light/dark), Zen mode
│   ├── app.js              # View switcher, IPC bridge to Rust, scroll synchronization, hotkeys
│   └── vendor/             # Offline dependencies (marked.min.js, highlight.min.js)
└── src-tauri/              # Rust application backend
    ├── Cargo.toml          # Rust dependencies (tauri, notify, rfd, serde)
    ├── tauri.conf.json     # Tauri v2 configuration (window size, identifiers, permissions)
    ├── build.rs            # Tauri build script
    ├── capabilities/       # Tauri v2 permission sets
    ├── icons/              # Application icon assets for packaging and desktop launchers
    └── src/
        ├── main.rs         # Application entry point
        └── lib.rs          # Core backend logic (IPC commands, CLI arg parsing, inotify watcher)
```

### Backend (Rust / `src-tauri`)
- **CLI File Argument**: Checks `std::env::args()` on startup (`mdv document.md`, `mdv ~/path/to/doc.md`, or `mdv /absolute/path.md`) and loads it automatically. Handles `--help` and `--version` terminal flags cleanly.
- **File Watcher**: `notify::RecommendedWatcher` watches the file's parent directory to handle atomic saves (write-and-rename) common in Unix editors. Emits `file-updated` events to the webview.
- **Native File Dialog**: `rfd::FileDialog` opens native GTK/portal file pickers without external plugin complexity (filters for `.md`, `.markdown`, `.mdown`, `.mkdn`, `.txt`).
- **IPC Handlers**:
  - `get_initial_file`: Retrieves the file passed via CLI or returns `None` (triggers welcome document).
  - `open_file_dialog`: Launches native file chooser and attaches the live-reload watcher.
  - `open_file_path`: Loads and watches an explicit path.
  - `open_link`: Handles secure link clicks (local relative/absolute files, external URLs, security checks preventing execution of scripts/binaries).
  - `get_app_version`: Returns the current application version dynamically from Cargo package metadata.
  - `set_window_title`: Synchronizes OS window title with current document name.
  - `toggle_fullscreen`: Handles native fullscreen for Zen reading mode.

### Frontend (`ui/`)
- **Offline & Bundled**: Does not make network requests; `marked.min.js` and `highlight.min.js` are vendored locally.
- **Scroll Sync**: When switching between Preview and Source with `Ctrl+E` or `Tab`, the viewport's relative scroll percentage is calculated and restored.
- **Theme System**: Responsive to system preference (`data-theme="auto"`) and supports manual overrides (`light` / `dark`) stored in `localStorage`.
- **History & Links**: Traverses navigation history (<kbd>Alt</kbd> + <kbd>←</kbd> / <kbd>Backspace</kbd> / Mouse Button 3) and routes local Markdown files directly in-app while opening external links and media in default desktop apps.

---

## 3. Common Development Tasks

The project uses a standard `Makefile`:

| Command | Description |
| :--- | :--- |
| `make help` | Print available Makefile targets |
| `make dev` | Build and run debug executable with default welcome document |
| `make run FILE=path/to/doc.md` | Run viewer with a specific markdown file |
| `make build` | Compile optimized release binary to `src-tauri/target/release/mdv` |
| `make check` | Run fast cargo type-check |
| `make test` | Run Rust test suite (`cargo test`) |
| `make install` | Install/update release binary only to `~/.local/bin/mdv` |
| `make install-desktop` | Install desktop shortcut, icon & file associations |
| `make associate-mime` | Set `mdv` as default application for Markdown files |
| `make uninstall` | Remove binary, desktop entry, and icon |
| `make clean` | Remove all compiled cargo build artifacts |
| `make deps-arch` | Print/install Arch/Manjaro required libraries |
| `make deps-mint` | Print/install Linux Mint/Ubuntu required libraries |

---

## 4. Key Conventions for Agents

1. **No Node Build Chains**: Do NOT add Webpack, Vite, React, or npm build steps unless specifically requested. Keep `ui/` static so that `cargo build` is the single source of truth.
2. **Preserve Single-Viewport Design**: Do NOT introduce a split-view / side-by-side preview. The core identity of `mdv` is distraction-free reading with a single viewport toggle.
3. **Robust File Watching**: When modifying filesystem watching logic in `src-tauri/src/lib.rs`, ensure atomic write / swap handling is preserved (editors like Vim/Neovim rename temporary files on save).
4. **Theme Contrast**: Ensure all Markdown elements (blockquotes, tables, inline code chips, task list checkboxes, and syntax highlighting) maintain high contrast and legibility in both light and dark themes.
