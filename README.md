# mdv — Native Linux Markdown Focus Viewer

**mdv** is a lightweight, distraction-free **Markdown Focus Reader** and viewer for Linux. Built in **100% pure Rust** using **egui** and **egui_commonmark** — zero WebKit, zero JavaScript, and zero browser engine overhead.

---

## Key Features

* **Pure Rust & egui**: Zero browser engine overhead (no WebKit), single self-contained native process, ~20–35 MB RAM footprint, and instantaneous launch (<20 ms).
* **Single-Viewport Toggle**: Toggle between rendered Markdown preview and raw source (<kbd>Ctrl</kbd> + <kbd>E</kbd> or <kbd>Tab</kbd>) inside the same window — no side-by-side split panes.
* **Scroll Position Sync**: Seamlessly preserves reading position and relative scroll offset across view toggles.
* **Responsive Column Width**: Adapts to full window width by default, with an instant toggle to a centered reading column (<kbd>W</kbd>).
* **Font Switching**: Toggle between clean proportional typography (**Inter NL**) and programmer's monospace (**JetBrains Mono NL**) (<kbd>F</kbd>) for unambiguous glyph discrimination (`0`/`O`, `1`/`l`/`I`).
* **Document Navigation History**: Seamlessly traverse previously viewed documents with <kbd>Alt</kbd> + <kbd>←</kbd>, <kbd>Backspace</kbd>, or mouse thumb back button (button 3 / `Extra1`).
* **Live Auto-Reload (Inotify)**: Real-time file watching updates the view immediately whenever you save in any text editor (Vim, Helix, Micro, VSCode), with full support for atomic write-and-rename swaps.
* **Syntax Highlighting**: Hardware-accelerated code block syntax highlighting.
* **Light and Dark Themes**: Automatically matches system dark/light preferences (`auto`) or manual cycling (<kbd>Ctrl</kbd> + <kbd>T</kbd>).
* **Zen Focus Mode**: Completely hides toolbar and window chrome (<kbd>F11</kbd> or <kbd>Z</kbd>), leaving pure text. Hover near the top edge to smoothly reveal controls.
* **CLI Support**: Open files directly from your terminal (`mdv document.md`, `mdv ~/notes.md`).

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> or <kbd>Tab</kbd> | **Toggle View (Preview / Markdown Source)** |
| <kbd>W</kbd> | **Toggle Width (Full Window / Centered Column)** |
| <kbd>F</kbd> | **Toggle Font (Inter NL / JetBrains Mono NL)** |
| <kbd>Alt</kbd> + <kbd>←</kbd>, <kbd>Backspace</kbd> or Mouse 3 | **Back in Document History** |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | **Cycle Theme (Auto / Light / Dark)** |
| <kbd>F11</kbd> or <kbd>Z</kbd> | **Zen / Fullscreen Mode** |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Open file via native file dialog |
| <kbd>Ctrl</kbd> + <kbd>+</kbd> / <kbd>-</kbd> | Zoom UI & font size in / out |
| <kbd>Ctrl</kbd> + <kbd>0</kbd> | Reset zoom to 100% |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Scroll up / down |
| <kbd>PgUp</kbd> / <kbd>PgDn</kbd> | Scroll page up / down |
| <kbd>Home</kbd> / <kbd>End</kbd> | Jump to top / bottom of document |
| <kbd>?</kbd> | Show keyboard shortcuts & help dialog |
| <kbd>Esc</kbd> | Close help / Exit Zen mode / Close application |

---

## System Requirements & Build

`mdv` compiles into a standalone, optimized native binary. It does not require WebKitGTK or Node.js!

### System Dependencies

* **Arch Linux / Manjaro:**
  ```bash
  make deps-arch
  # or manually: sudo pacman -S --needed base-devel libxkbcommon
  ```

* **Linux Mint / Ubuntu / Debian:**
  ```bash
  make deps-mint
  # or manually: sudo apt update && sudo apt install -y build-essential libxkbcommon-dev pkg-config
  ```

### Build & Makefile Targets

The following commands are available in the project directory:

```bash
# Display all available targets:
make help

# Build optimized release binary:
make build

# Run debug build with default welcome document:
make dev

# Run with a specific file:
make run FILE=SAMPLE.md

# Quick type checking (cargo check):
make check

# Run Rust test suite:
make test

# Remove compiled binaries and cargo build artifacts:
make clean
```

---

## System Installation & File Associations

To launch `mdv` anywhere from your terminal or open Markdown files with a double-click in your file manager:

### Method 1: Automatic (Recommended)

```bash
# 1. Compile and install binary to ~/.local/bin/mdv:
make install

# 2. Create desktop entry (~/.local/share/applications), install icon & register MIME associations:
make install-desktop

# 3. (Optional) Set mdv as default application for .md files:
make associate-mime

# 4. Uninstall binary, desktop shortcut, and icon:
make uninstall
```

---

### Method 2: Manual Configuration

#### Step 1: Copy executable binary
```bash
mkdir -p ~/.local/bin
cp target/release/mdv ~/.local/bin/mdv
chmod +x ~/.local/bin/mdv
```

#### Step 2: Create desktop launcher (`.desktop`)
Create `~/.local/share/applications/mdv.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=mdv
GenericName=Markdown Viewer
Comment=Minimalist Markdown Focus Reader
Exec=/home/<username>/.local/bin/mdv %U
Icon=mdv
Terminal=false
Categories=Utility;Viewer;Office;
MimeType=text/markdown;text/x-markdown;
StartupNotify=true
```
*(Replace `<username>` with your actual Linux user name)*

Then update the desktop database:
```bash
update-desktop-database ~/.local/share/applications/
```

#### Step 3: Register `mdv` as default handler for `.md` files
```bash
gio mime text/markdown mdv.desktop
gio mime text/x-markdown mdv.desktop
xdg-mime default mdv.desktop text/markdown
xdg-mime default mdv.desktop text/x-markdown
```

---

## Archived Version (`legacy_tauri/`) — READ-ONLY

The `legacy_tauri/` folder contains an earlier prototype of `mdv`. **This version is completely deprecated, no longer maintained or supported, and the directory is strictly read-only — no changes should ever be made there.**

### How Does Legacy Differ from the Current Version?

| Aspect | Current Version (`mdv` / root) | Legacy Version (`legacy_tauri/`) |
| :--- | :--- | :--- |
| **Architecture** | **100% Pure Rust** (`egui` + `eframe`) | **Tauri v2** + Rust backend + web frontend (HTML/CSS/JS) |
| **Rendering Engine** | Direct native vector / GPU rendering (`egui_commonmark`) | Browser webview engine (**WebKitGTK**) |
| **Technology Stack** | Standalone native Rust binary, zero browser stack | Hybrid: Rust, Tauri IPC, DOM tree, JavaScript, CSS stylesheets |
| **Markdown Parsing** | Direct CommonMark parsing into native `egui` widgets | External JavaScript libraries (`marked.min.js`, `highlight.min.js`) |
| **System Dependencies** | Standard Linux system libraries (e.g. `libxkbcommon`) | Heavy WebKit ecosystem: `webkit2gtk-4.1`, `libsoup-3.0`, `gtk3`, `javascriptcoregtk-4.1` |
| **Memory Footprint (RAM)** | **~20–35 MB** | **~100+ MB** (multi-process browser runtime and JS engine) |
| **Startup Latency** | Instantaneous (<20 ms) | Noticeable delay initializing WebKit and executing scripts |
| **Status** | **Actively developed main version** | **Archived, unsupported, read-only (do not modify)** |

> [!CAUTION]
> **The `legacy_tauri/` directory is strictly archival and read-only.** No bug fixes, refactoring, or modifications should be performed there. All ongoing development and codebase improvements take place exclusively in the root pure-Rust `egui` project.

---

## Project Structure

```
mdv/
├── Makefile                # Build, installation, and MIME association scripts
├── Cargo.toml              # Rust crate dependencies (eframe, egui, egui_commonmark, notify, rfd)
├── README.md               # User guide and setup documentation
├── AGENTS.md               # Architecture and guidelines for human & AI contributors
├── SAMPLE.md               # Demonstration markdown document
├── icons/                  # Application icon assets for desktop launchers
├── legacy_tauri/           # ARCHIVED & READ-ONLY: Deprecated Tauri + WebKitGTK version
└── src/
    ├── main.rs             # CLI entry point, window setup, and argument parsing
    ├── app.rs              # Application state, view switcher, shortcuts, and UI
    ├── watcher.rs          # Background filesystem watcher (inotify) with atomic save handling
    └── welcome.rs          # Embedded welcome and shortcut guide document
```

---

## License

This project is licensed under the MIT / Apache 2.0 license.
