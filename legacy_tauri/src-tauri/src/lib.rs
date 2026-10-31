use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileData {
    pub path: String,
    pub filename: String,
    pub content: String,
}

pub struct AppState {
    pub current_file: Arc<Mutex<Option<PathBuf>>>,
    pub watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
}

fn resolve_path(arg: &str) -> PathBuf {
    if arg.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(&arg[2..]);
        }
    } else if arg == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(arg)
}

fn watch_target_file(app: &AppHandle, state: &AppState, target: &Path) -> Result<(), String> {
    let target = match target.canonicalize() {
        Ok(p) => p,
        Err(_) => target.to_path_buf(),
    };

    let mut watcher_lock = state.watcher.lock().map_err(|e| e.to_string())?;
    *watcher_lock = None;

    let app_handle = app.clone();
    let watched_file = target.clone();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let is_mod = match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => true,
                    _ => false,
                };

                if is_mod {
                    // Check if the modified file matches our watched file
                    let matches = event.paths.is_empty()
                        || event.paths.iter().any(|p| {
                            p == &watched_file
                                || p.file_name() == watched_file.file_name()
                        });

                    if matches {
                        // Allow file writes / atomic renames to settle
                        std::thread::sleep(std::time::Duration::from_millis(60));
                        if let Ok(content) = std::fs::read_to_string(&watched_file) {
                            let filename = watched_file
                                .file_name()
                                .and_then(|f| f.to_str())
                                .unwrap_or("untitled.md")
                                .to_string();
                            let path = watched_file.to_string_lossy().to_string();
                            let data = FileData {
                                path,
                                filename,
                                content,
                            };
                            let _ = app_handle.emit("file-updated", data);
                        }
                    }
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| e.to_string())?;

    // Watch parent directory to handle atomic saves/renames common in Linux editors
    let watch_dir = target.parent().unwrap_or(&target);
    watcher
        .watch(watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    *watcher_lock = Some(watcher);
    if let Ok(mut current) = state.current_file.lock() {
        *current = Some(target);
    }

    Ok(())
}

fn read_file_data(path: &Path) -> Result<FileData, String> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        format!("Failed to read file '{}': {}", path.display(), e)
    })?;

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let filename = canonical
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("document.md")
        .to_string();
    let full_path = canonical.to_string_lossy().to_string();

    Ok(FileData {
        path: full_path,
        filename,
        content,
    })
}

#[tauri::command]
fn get_initial_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<FileData>, String> {
    // Check command line args for a file path
    let args: Vec<String> = std::env::args().collect();
    for arg in args.into_iter().skip(1) {
        if !arg.starts_with('-') {
            let p = resolve_path(&arg);
            if p.is_file() {
                let file_data = read_file_data(&p)?;
                let _ = watch_target_file(&app, &state, &p);
                return Ok(Some(file_data));
            } else {
                // If user provided a path that doesn't exist
                return Ok(Some(FileData {
                    path: arg.clone(),
                    filename: "File Not Found".to_string(),
                    content: format!(
                        "# File Not Found\n\nThe specified file was not found:\n\n```text\n{}\n```\n\nPlease check that the file path and name are correct.",
                        arg
                    ),
                }));
            }
        }
    }
    Ok(None)
}

#[tauri::command]
fn open_file_dialog(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<FileData>, String> {
    let picked = rfd::FileDialog::new()
        .add_filter(
            "Markdown / Text",
            &["md", "markdown", "mdown", "mkdn", "txt"],
        )
        .pick_file();

    if let Some(path) = picked {
        let file_data = read_file_data(&path)?;
        let _ = watch_target_file(&app, &state, &path);
        return Ok(Some(file_data));
    }

    Ok(None)
}

#[tauri::command]
fn open_file_path(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<FileData, String> {
    let p = resolve_path(&path);
    let file_data = read_file_data(&p)?;
    let _ = watch_target_file(&app, &state, &p);
    Ok(file_data)
}

fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(ch);
        }
    }
    result
}

#[tauri::command]
fn open_link(
    app: AppHandle,
    state: State<'_, AppState>,
    link: String,
) -> Result<Option<FileData>, String> {
    let trimmed = link.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    // 1. Web / Internet Links (http, https, mailto)
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("mailto:")
    {
        std::process::Command::new("xdg-open")
            .arg(trimmed)
            .spawn()
            .map_err(|e| format!("Failed to open link: {}", e))?;
        return Ok(None);
    }

    // 2. Local filesystem links
    let without_file_scheme = if let Some(stripped) = trimmed.strip_prefix("file://") {
        stripped
    } else {
        trimmed
    };

    // Strip in-file anchor if present
    let clean_path = if let Some(idx) = without_file_scheme.find('#') {
        &without_file_scheme[..idx]
    } else {
        without_file_scheme
    };

    if clean_path.is_empty() {
        return Ok(None);
    }

    let decoded = url_decode(clean_path);

    // Resolve relative path against currently opened document directory
    let target_path = if decoded.starts_with('/') || decoded.starts_with('~') {
        resolve_path(&decoded)
    } else {
        let base_dir = {
            let cur = state.current_file.lock().map_err(|e| e.to_string())?;
            cur.as_ref()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        };
        base_dir.join(&decoded)
    };

    let canonical = match target_path.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            return Err(format!("File does not exist: {}", target_path.display()));
        }
    };

    if !canonical.is_file() {
        return Err(format!("Target is not a regular file: {}", canonical.display()));
    }

    // Security check: Never execute binaries or scripts
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    const BLOCKED_EXTENSIONS: &[&str] = &[
        "sh", "bash", "zsh", "bin", "run", "appimage", "desktop",
        "exe", "bat", "cmd", "vbs", "ps1", "com", "msi"
    ];
    if BLOCKED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!(
            "Blocked for security: Opening executable or script file (.{}) is not allowed.",
            ext
        ));
    }

    // Markdown / text files -> open in mdv and attach live watcher
    const MD_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkdn", "txt"];
    if MD_EXTENSIONS.contains(&ext.as_str()) {
        let file_data = read_file_data(&canonical)?;
        let _ = watch_target_file(&app, &state, &canonical);
        Ok(Some(file_data))
    } else {
        // Other non-executable files (PDFs, images, etc.) -> open in default system application
        std::process::Command::new("xdg-open")
            .arg(&canonical)
            .spawn()
            .map_err(|e| format!("Failed to open file in system application: {}", e))?;
        Ok(None)
    }
}

#[tauri::command]
fn set_window_title(app: AppHandle, title: String) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_title(&title).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn toggle_fullscreen(app: AppHandle) -> Result<bool, String> {
    if let Some(win) = app.get_webview_window("main") {
        let is_fs = win.is_fullscreen().map_err(|e| e.to_string())?;
        win.set_fullscreen(!is_fs).map_err(|e| e.to_string())?;
        return Ok(!is_fs);
    }
    Ok(false)
}

#[tauri::command]
fn get_app_version() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        current_file: Arc::new(Mutex::new(None)),
        watcher: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_initial_file,
            open_file_dialog,
            open_file_path,
            open_link,
            set_window_title,
            toggle_fullscreen,
            get_app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
