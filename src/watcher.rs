use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct FileUpdate {
    pub path: PathBuf,
    pub content: String,
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn watch(
        target: &Path,
        tx: Sender<FileUpdate>,
        egui_ctx: egui::Context,
    ) -> Result<Self, String> {
        let canonical_target = target.canonicalize().unwrap_or_else(|_| target.to_path_buf());
        let watched_file = canonical_target.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let is_relevant = match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) => true,
                        _ => false,
                    };

                    if is_relevant {
                        let matches = event.paths.is_empty()
                            || event.paths.iter().any(|p| {
                                p == &watched_file
                                    || p.file_name() == watched_file.file_name()
                            });

                        if matches {
                            // Give disk writes & atomic swap renames time to settle
                            std::thread::sleep(Duration::from_millis(60));
                            if let Ok(content) = std::fs::read_to_string(&watched_file) {
                                let _ = tx.send(FileUpdate {
                                    path: watched_file.clone(),
                                    content,
                                });
                                egui_ctx.request_repaint();
                            }
                        }
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        let watch_dir = canonical_target.parent().unwrap_or(&canonical_target);
        watcher
            .watch(watch_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch directory '{}': {}", watch_dir.display(), e))?;

        Ok(Self { _watcher: watcher })
    }
}
