pub const WELCOME_DOC: &str = r#"# Welcome to mdv v0.2.1

**mdv** is a native, distraction-free **Markdown Focus Viewer** written purely in Rust.

---

### Key Features

* **Instant View Toggle**: Switch between **Rendered Preview** and **Raw Source** in the same window (no distracting side-by-side panels).
* **Flexible Width**: Automatically adapts to full window width or centers into a clean reading column (press <kbd>W</kbd>).
* **Live Auto-Reload**: Whenever you save your markdown file in Vim, Helix, or any editor, mdv instantly updates on screen.
* **Smart File Links**: Click local markdown links, images, PDFs, or web URLs with built-in navigation history (<kbd>Alt</kbd> + <kbd>←</kbd>).
* **Zen Focus Mode**: Strip away all window chrome and focus purely on reading.
* **Light & Dark Themes**: System-adaptive with custom handcrafted contrast palettes.
* **Pure Rust & Hardware Accelerated**: Powered directly by `egui` and `wgpu`/OpenGL with zero webview overhead.

---

### Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> or <kbd>Tab</kbd> | **Toggle Preview / Source** |
| <kbd>W</kbd> | **Toggle Full Width / Centered Column** |
| <kbd>F</kbd> | **Toggle Font (Inter / JetBrains Mono)** |
| <kbd>Alt</kbd> + <kbd>←</kbd> or <kbd>Backspace</kbd> | **Back in Document History** |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | **Open Markdown File** |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | **Cycle Theme (Auto / Light / Dark)** |
| <kbd>F11</kbd> or <kbd>Z</kbd> | **Toggle Zen Focus Mode** |
| <kbd>Ctrl</kbd> + <kbd>+</kbd> / <kbd>-</kbd> | Zoom Font Size (+ / -) |
| <kbd>Ctrl</kbd> + <kbd>0</kbd> | Reset Font Size (16px) |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Scroll Up / Down |
| <kbd>PgUp</kbd> / <kbd>PgDn</kbd> | Scroll Page Up / Down |
| <kbd>Home</kbd> / <kbd>End</kbd> | Jump to Top / Bottom |
| <kbd>?</kbd> | Show Keyboard Shortcuts & About |
| <kbd>Esc</kbd> | Close Modal / Exit Focus Mode / Close mdv |

---

### Code Syntax Example

```rust
fn main() {
    let focus = "100%";
    println!("Distraction-free reading with {} focus in pure Rust!", focus);
}
```

> *"Simplicity is the ultimate sophistication."* — Leonardo da Vinci

*Tip: You can launch from the terminal with `mdv my-document.md` or click **Open** above!*
"#;
