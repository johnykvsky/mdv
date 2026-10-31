# AGENTS.md — Developer & AI Agent Guide for `mdv`

Welcome to **`mdv`** (Markdown Viewer), a native Linux, distraction-free **Markdown Focus Reader** written in 100% pure Rust using `egui` and `egui_commonmark`.

---

## 1. Project Philosophy & Core Requirements

- **100% Pure Rust & Zero WebKit**: No webview, no JavaScript, no CSS, and no runtime browser engines. Minimal footprint (~25 MB RAM) with instantaneous startup.
- **Distraction-Free Focus & Responsive Width**: Clean typography with flexible layout. Adapts to full window width by default, with option to toggle to a centered reading column (<kbd>W</kbd>).
- **Single-Viewport Toggle**: Toggle between **Rendered Preview** and **Raw Source** inside the *same main window* (no side-by-side panels). Relative scroll position is preserved across toggles. Source view renders line-by-line with clean non-selectable line numbers and text wrapping (<kbd>Ctrl</kbd> + <kbd>E</kbd> or <kbd>Tab</kbd>).
- **Font Switching**: Toggle between modern clean sans-serif (**Inter**) and programmer's monospace (**JetBrains Mono**) (<kbd>F</kbd>) for unambiguous glyph discrimination (`0`/`O`, `1`/`l`/`I`).
- **Document Navigation History**: Seamlessly traverse previously viewed documents with <kbd>Alt</kbd> + <kbd>←</kbd>, <kbd>Backspace</kbd>, or mouse thumb button (button 3 / `Extra1`).
- **Live Auto-Reload**: Watches files on disk via `notify`. When the user edits and saves in any text editor (Vim, Helix, Micro, VSCode), `mdv` updates the view instantly. Handles atomic saves (write-and-rename) common in Linux editors.
- **Font & UI Zoom Controls**: Dynamic vector scaling with <kbd>Ctrl</kbd> + <kbd>+</kbd> / <kbd>-</kbd> and reset with <kbd>Ctrl</kbd> + <kbd>0</kbd>.
- **Portability Across Linux Distros**: Runs on Wayland and X11 without requiring WebKitGTK. Standard graphics libraries (OpenGL/Vulkan) only.

---

## 2. Architecture Overview

```
mdv/
├── Makefile                # Primary developer task runner (build, run, install, deps)
├── Cargo.toml              # Rust crate dependencies (eframe, egui, egui_commonmark, notify, rfd)
├── README.md               # User guide, keybindings, and desktop integration instructions
├── AGENTS.md               # Architecture and guidelines for human & AI contributors
├── SAMPLE.md               # Sample demonstration markdown document
├── .gitignore              # Rust targets, editor and OS artifacts
├── icons/                  # Application icon assets for packaging and desktop launchers
├── legacy_tauri/           # ARCHIVED & READ-ONLY: Deprecated Tauri v2 + WebKitGTK implementation
└── src/
    ├── main.rs             # CLI argument parsing, window viewport setup, entry point
    ├── app.rs              # Application state, egui App implementation, view switcher, shortcuts
    ├── watcher.rs          # Background file watching via notify (handles atomic saves & renames)
    └── welcome.rs          # Embedded welcome & shortcut guide document
```

### Core Components
- **CLI Handling (`src/main.rs`)**: Checks `std::env::args()` on startup (`mdv doc.md`, `mdv ~/path/to/doc.md`, or `mdv /absolute/path.md`) and loads it automatically. Handles `--help` and `--version` terminal flags cleanly.
- **Main View & Event Loop (`src/app.rs`)**:
  - `MdvApp` implements `eframe::App`.
  - Manages view toggles (Preview vs Source), width modes (Full vs Centered), theme cycles (Auto, Light, Dark), and font modes (Sans vs Mono).
  - Intercepts link clicks via `egui::OutputCommand::OpenUrl`: opens web links in the default browser, loads markdown files in-app, opens other media in system tools, and rejects executable scripts/binaries for security.
  - Implements scroll ratio tracking across view toggles.
- **File Watcher (`src/watcher.rs`)**: `notify::RecommendedWatcher` watches the file's parent directory to handle atomic saves (write-and-rename) common in Unix editors. Emits file updates over an `mpsc::channel` and triggers `ctx.request_repaint()`.

---

## 3. Common Development Tasks

The project uses a standard `Makefile`:

| Command | Description |
| :--- | :--- |
| `make help` | Print available Makefile targets |
| `make dev` | Build and run debug executable with default welcome document |
| `make run FILE=path/to/doc.md` | Run viewer with a specific markdown file |
| `make build` | Compile optimized release binary to `target/release/mdv` |
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

1. **Keep It 100% Pure Rust**: Do NOT introduce webviews, Tauri, Electron, Node.js, WebKit, or HTML/JS build chains.
2. **Preserve Single-Viewport Design**: Do NOT introduce a split-view / side-by-side preview. The core identity of `mdv` is distraction-free reading with a single viewport toggle.
3. **Robust File Watching**: When modifying filesystem watching logic in `src/watcher.rs`, ensure atomic write / swap handling is preserved (editors like Vim/Neovim rename temporary files on save).
4. **Theme Contrast**: Ensure all Markdown elements (blockquotes, tables, inline code chips, task list checkboxes, and syntax highlighting) maintain high contrast and legibility in both light and dark themes.
5. **Do Not Touch `legacy_tauri/`**: The `legacy_tauri/` directory is an immutable, read-only historic archive. Never apply fixes, refactorings, or modifications to any file inside this folder.

---

## 5. Legacy Tauri Archive (`legacy_tauri/`) — Read-Only & Unsupported

The `legacy_tauri/` folder contains the original prototype implementation of `mdv`, created using **Tauri v2** and **WebKitGTK**. It has been permanently superseded by the pure Rust `egui` architecture.

> [!CAUTION]
> **STRICTLY READ-ONLY & DEPRECATED**:
> - **No changes or modifications** should ever be made within `legacy_tauri/`.
> - Do not attempt to fix bugs, refactor, or run builds in that directory.
> - All development, bug fixing, and feature additions must take place exclusively in the root workspace (`src/`).

### Technical Comparison: Current vs Legacy

| Dimension | Current `mdv` (Root Workspace) | Legacy Version (`legacy_tauri/`) |
| :--- | :--- | :--- |
| **Framework / Core** | 100% Pure Rust (`eframe` / `egui`) | Tauri v2 + Rust backend |
| **Display Engine** | Direct GPU / software vector rendering (`egui_commonmark`) | Embedded browser webview (**WebKitGTK**) |
| **Frontend Stack** | None (Immediate-mode native GUI widgets) | Web stack (`ui/index.html`, `ui/style.css`, `ui/app.js`) |
| **Markdown Rendering** | Native Rust CommonMark parser into UI elements | Web JavaScript libraries (`marked.min.js`, `highlight.min.js`) |
| **System Dependencies** | Standard Linux graphics libraries (`libxkbcommon`) | Heavy WebKitGTK stack (`webkit2gtk-4.1`, `libsoup3`, `gtk3`, `javascriptcoregtk`) |
| **RAM Consumption** | **~20–35 MB** | **~100+ MB** (due to WebKit multi-process runtime & JS engine) |
| **Startup Latency** | Instantaneous (<20 ms) | Noticeable lag during WebKitGTK and JS runtime init |
| **Status** | **Active & supported** | **Unsupported, frozen archive (read-only)** |
