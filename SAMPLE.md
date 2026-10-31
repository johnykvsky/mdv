# Focus Reading with `mdv`

> "Perfection is achieved, not when there is nothing more to add, but when there is nothing left to take away."  
> — Antoine de Saint-Exupéry

`mdv` is designed for pure focus reading on Linux. It gives you an elegant, centered reading column with zero distractions.

---

## Quick Controls

* **Toggle Preview / Source**: Press <kbd>Ctrl</kbd> + <kbd>E</kbd> or <kbd>Tab</kbd> to switch between rendered view and raw Markdown.
* **Zen / Focus Mode**: Press <kbd>F11</kbd> or <kbd>Z</kbd> to hide all toolbar chrome. Move mouse to the very top edge to reveal controls.
* **Theme Switching**: Press <kbd>Ctrl</kbd> + <kbd>T</kbd> to cycle between **Auto (System)**, **Light**, and **Dark** modes.
* **Live Auto-Reload**: Keep this window open, edit this file in your favorite terminal editor (`vim SAMPLE.md` or `nano SAMPLE.md`), save it, and watch the preview update immediately!

---

## Formatted Elements

### Task Checklist
- [x] Native Linux performance
- [x] Single-viewport view switching
- [x] Auto-reloading on disk writes
- [x] High-contrast light & dark themes
- [ ] Export to PDF (planned)

### Clean Tables

| Toolkit | Binary Size | Memory (RAM) | Distro Portability |
| :--- | :--- | :--- | :--- |
| **mdv (Pure Rust + egui)** | ~25 MB | ~25 MB | Standalone (Zero WebKit) |
| **Electron-based** | ~140 MB | ~280 MB | Moderate |
| **Pure GTK4-rs** | ~20 MB | ~45 MB | Distribution-bound |

---

## Code Highlighting

Here is an example Rust snippet showing how `mdv` watches your files:

```rust
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

fn watch_document(path: &Path) -> notify::Result<()> {
    let mut watcher = RecommendedWatcher::new(|res| {
        if let Ok(event) = res {
            println!("File changed on disk: {:?}", event);
        }
    }, notify::Config::default())?;

    watcher.watch(path, RecursiveMode::NonRecursive)?;
    Ok(())
}
```

```python
# Minimalist Python example
def focus_mode(enabled: bool) -> str:
    return "Distraction-free reading active" if enabled else "Normal mode"

print(focus_mode(True))
```

---

## Links and Notes

1. Check the [AGENTS.md](AGENTS.md) guide for architecture details.
2. Run `make help` in your terminal to see all available tasks.
