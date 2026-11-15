use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use egui::{
    Color32, CornerRadius, FontId, Id, Key, Margin, OutputCommand, PointerButton, RichText, Stroke, Ui, Vec2,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::icons;
use crate::watcher::{FileUpdate, FileWatcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Preview,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthMode {
    Full,
    Centered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontMode {
    Sans,
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub struct FileData {
    pub path: Option<PathBuf>,
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub path: Option<PathBuf>,
    pub filename: String,
    pub content: String,
    pub scroll_ratio: f32,
}

#[derive(Default, Debug, Clone)]
pub struct SearchState {
    pub is_open: bool,
    pub query: String,
    pub active_index: usize,
    pub total_matches: usize,
    pub focus_requested: bool,
    pub should_scroll_source: bool,
    pub matches_source: Vec<std::ops::Range<usize>>,
}

impl SearchState {
    pub fn reset(&mut self, cache: &mut CommonMarkCache) {
        self.is_open = false;
        self.query.clear();
        self.active_index = 0;
        self.total_matches = 0;
        self.focus_requested = false;
        self.should_scroll_source = false;
        self.matches_source.clear();
        let source_id = egui::Id::unique("mdv_markdown_view");
        let sc = cache.search_cache_mut(&source_id);
        sc.search_query.clear();
        sc.search_ranges.clear();
        sc.active_match = None;
        sc.active_search_range = None;
    }
}

pub struct MdvApp {
    pub current_file: Option<FileData>,
    pub view_mode: ViewMode,
    pub width_mode: WidthMode,
    pub font_mode: FontMode,
    pub theme_mode: ThemeMode,
    pub font_size: f32,
    pub word_wrap: bool,
    pub is_zen_mode: bool,
    pub show_shortcuts_modal: bool,
    pub toast: Option<(String, Instant)>,
    pub focus_frames: u8,

    // Search
    pub search_state: SearchState,

    // Navigation & history
    pub history: Vec<HistoryEntry>,
    pub current_scroll_ratio: f32,
    pub pending_scroll_ratio: Option<f32>,
    pub current_scroll_offset: f32,
    pub max_scroll_offset: f32,
    pub viewport_height: f32,
    pub pending_scroll_offset: Option<f32>,

    // File watcher
    pub watcher: Option<FileWatcher>,
    pub file_rx: Receiver<FileUpdate>,
    pub file_tx: Sender<FileUpdate>,

    // Markdown cache
    pub commonmark_cache: CommonMarkCache,
}

impl MdvApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_file: Option<PathBuf>) -> Self {
        // Install custom embedded typography: Source Sans 3 and JetBrains Mono NL (Code)
        setup_custom_fonts(&cc.egui_ctx);

        // Install image loaders for egui_extras
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let (file_tx, file_rx) = channel();

        let mut app = Self {
            current_file: None,
            view_mode: ViewMode::Preview,
            width_mode: WidthMode::Full,
            font_mode: FontMode::Sans,
            theme_mode: ThemeMode::Auto,
            font_size: 16.0,
            word_wrap: true,
            is_zen_mode: false,
            show_shortcuts_modal: false,
            toast: None,
            focus_frames: 20,
            search_state: SearchState::default(),
            history: Vec::new(),
            current_scroll_ratio: 0.0,
            pending_scroll_ratio: None,
            current_scroll_offset: 0.0,
            max_scroll_offset: 0.0,
            viewport_height: 600.0,
            pending_scroll_offset: None,
            watcher: None,
            file_rx,
            file_tx,
            commonmark_cache: CommonMarkCache::default(),
        };

        if let Some(path) = initial_file {
            let _ = app.load_file(&path, &cc.egui_ctx);
        }

        app
    }

    pub fn set_toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    pub fn load_file(&mut self, path: &Path, ctx: &egui::Context) -> Result<(), String> {
        self.search_state.reset(&mut self.commonmark_cache);
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !canonical.is_file() {
            self.current_file = Some(FileData {
                path: Some(canonical.clone()),
                filename: "File Not Found".to_string(),
                content: format!(
                    "# File Not Found\n\nThe specified file was not found:\n\n```text\n{}\n```\n\nPlease check that the file path and name are correct.",
                    canonical.display()
                ),
            });
            self.watcher = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "File Not Found - mdv v{}",
                env!("CARGO_PKG_VERSION")
            )));
            return Err(format!("File not found: {}", canonical.display()));
        }

        let content = std::fs::read_to_string(&canonical)
            .map_err(|e| format!("Failed to read '{}': {}", canonical.display(), e))?;

        let filename = canonical
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("document.md")
            .to_string();

        self.current_file = Some(FileData {
            path: Some(canonical.clone()),
            filename: filename.clone(),
            content,
        });

        // Reset scroll position to top for newly opened file
        self.current_scroll_offset = 0.0;
        self.pending_scroll_offset = Some(0.0);
        self.current_scroll_ratio = 0.0;
        self.pending_scroll_ratio = None;

        // Attach live file watcher
        match FileWatcher::watch(&canonical, self.file_tx.clone(), ctx.clone()) {
            Ok(watcher) => {
                self.watcher = Some(watcher);
            }
            Err(e) => {
                eprintln!("Warning: Failed to attach file watcher: {}", e);
                self.watcher = None;
            }
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{} - mdv v{}",
            filename,
            env!("CARGO_PKG_VERSION")
        )));
        Ok(())
    }

    pub fn push_history(&mut self) {
        if let Some(ref current) = self.current_file {
            self.history.push(HistoryEntry {
                path: current.path.clone(),
                filename: current.filename.clone(),
                content: current.content.clone(),
                scroll_ratio: self.current_scroll_ratio,
            });
            if self.history.len() > 50 {
                self.history.remove(0);
            }
        }
    }

    pub fn go_back(&mut self, ctx: &egui::Context) -> bool {
        if let Some(prev) = self.history.pop() {
            if let Some(path) = prev.path {
                if let Ok(()) = self.load_file(&path, ctx) {
                    self.pending_scroll_ratio = Some(prev.scroll_ratio);
                    self.pending_scroll_offset = None;
                    self.set_toast(format!("Returned to {}", prev.filename));
                    return true;
                }
            } else {
                self.current_file = Some(FileData {
                    path: None,
                    filename: prev.filename.clone(),
                    content: prev.content,
                });
                self.watcher = None;
                self.pending_scroll_ratio = Some(prev.scroll_ratio);
                self.pending_scroll_offset = None;
                self.set_toast(format!("Returned to {}", prev.filename));
                return true;
            }
        } else {
            self.set_toast("No previous document in history");
        }
        false
    }

    pub fn open_file_dialog(&mut self, ctx: &egui::Context) {
        let picked = rfd::FileDialog::new()
            .add_filter("Markdown / Text", &["md", "markdown", "mdown", "mkdn", "txt"])
            .pick_file();

        if let Some(path) = picked {
            self.push_history();
            if let Ok(()) = self.load_file(&path, ctx) {
                if let Some(ref file) = self.current_file {
                    self.set_toast(format!("Loaded {}", file.filename));
                }
            }
        }
    }

    pub fn handle_link_click(&mut self, link: &str, ctx: &egui::Context) {
        let trimmed = link.trim();
        if trimmed.is_empty() {
            return;
        }

        // 1. In-document anchor link
        if trimmed.starts_with('#') {
            if let Some(target) = trimmed.strip_prefix('#') {
                let decoded = urlencoding::decode(target)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| target.to_string());
                self.commonmark_cache.scroll_to_heading(&Id::NULL, Some(decoded));
                ctx.request_repaint();
            }
            return;
        }

        // 2. Web link
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("mailto:")
        {
            if let Err(e) = open::that(trimmed) {
                self.set_toast(format!("Failed to open link: {}", e));
            } else {
                self.set_toast("Opened in browser");
            }
            return;
        }

        // 3. Local filesystem links
        let without_file_scheme = trimmed.strip_prefix("file://").unwrap_or(trimmed);
        let anchor_opt = without_file_scheme.find('#').map(|idx| &without_file_scheme[idx + 1..]);
        let clean_path = if let Some(idx) = without_file_scheme.find('#') {
            &without_file_scheme[..idx]
        } else {
            without_file_scheme
        };

        if clean_path.is_empty() {
            return;
        }

        let decoded = urlencoding::decode(clean_path)
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| clean_path.to_string());

        let base_dir = self
            .current_file
            .as_ref()
            .and_then(|f| f.path.as_ref())
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let target_path = if decoded.starts_with('/') || decoded.starts_with('~') {
            resolve_path(&decoded)
        } else {
            base_dir.join(&decoded)
        };

        let canonical = match target_path.canonicalize() {
            Ok(c) => c,
            Err(_) => {
                self.set_toast(format!("File does not exist: {}", target_path.display()));
                return;
            }
        };

        if !canonical.is_file() {
            self.set_toast(format!("Target is not a file: {}", canonical.display()));
            return;
        }

        // Security check: Block scripts and binaries
        let ext = canonical
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        const BLOCKED_EXTENSIONS: &[&str] = &[
            "sh", "bash", "zsh", "bin", "run", "appimage", "desktop", "exe", "bat", "cmd",
            "vbs", "ps1", "com", "msi",
        ];
        if BLOCKED_EXTENSIONS.contains(&ext.as_str()) {
            self.set_toast(format!(
                "Blocked for security: Opening .{ext} file is not allowed"
            ));
            return;
        }

        // Markdown files -> open in mdv
        const MD_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkdn", "txt"];
        if MD_EXTENSIONS.contains(&ext.as_str()) {
            self.push_history();
            if let Ok(()) = self.load_file(&canonical, ctx) {
                if let Some(anchor) = anchor_opt {
                    let decoded_anchor = urlencoding::decode(anchor)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| anchor.to_string());
                    self.commonmark_cache.scroll_to_heading(&Id::NULL, Some(decoded_anchor));
                }
                if let Some(ref file) = self.current_file {
                    self.set_toast(format!("Opened {}", file.filename));
                }
            }
        } else {
            // Other files (images, PDFs, documents) -> open in system default app
            if let Err(e) = open::that(&canonical) {
                self.set_toast(format!("Failed to open file: {}", e));
            } else {
                self.set_toast("Opened in system application");
            }
        }
    }

    pub fn toggle_view(&mut self) {
        self.pending_scroll_ratio = Some(self.current_scroll_ratio);
        self.search_state.reset(&mut self.commonmark_cache);
        self.view_mode = match self.view_mode {
            ViewMode::Preview => ViewMode::Source,
            ViewMode::Source => ViewMode::Preview,
        };
    }

    pub fn toggle_width(&mut self) {
        self.width_mode = match self.width_mode {
            WidthMode::Full => {
                self.set_toast("Width: Centered Column");
                WidthMode::Centered
            }
            WidthMode::Centered => {
                self.set_toast("Width: Full Window");
                WidthMode::Full
            }
        };
    }

    pub fn toggle_word_wrap(&mut self) {
        self.word_wrap = !self.word_wrap;
        self.set_toast(if self.word_wrap {
            "Word Wrap: ON"
        } else {
            "Word Wrap: OFF"
        });
    }

    pub fn toggle_font(&mut self) {
        self.font_mode = match self.font_mode {
            FontMode::Sans => {
                self.set_toast("Font: JetBrains Mono NL");
                FontMode::Mono
            }
            FontMode::Mono => {
                self.set_toast("Font: Source Sans 3");
                FontMode::Sans
            }
        };
    }

    pub fn cycle_theme(&mut self, ctx: &egui::Context) {
        self.theme_mode = match self.theme_mode {
            ThemeMode::Auto => {
                self.set_toast("Theme: Light");
                ThemeMode::Light
            }
            ThemeMode::Light => {
                self.set_toast("Theme: Dark");
                ThemeMode::Dark
            }
            ThemeMode::Dark => {
                self.set_toast("Theme: System Auto");
                ThemeMode::Auto
            }
        };
        self.apply_theme(ctx);
    }

    pub fn apply_theme(&self, ctx: &egui::Context) {
        match self.theme_mode {
            ThemeMode::Auto => {
                ctx.set_visuals(egui::Visuals::default());
            }
            ThemeMode::Light => {
                let mut visuals = egui::Visuals::light();
                visuals.panel_fill = Color32::from_rgb(252, 252, 253);
                visuals.window_fill = Color32::from_rgb(255, 255, 255);
                ctx.set_visuals(visuals);
            }
            ThemeMode::Dark => {
                let mut visuals = egui::Visuals::dark();
                visuals.panel_fill = Color32::from_rgb(22, 22, 26);
                visuals.window_fill = Color32::from_rgb(26, 26, 32);
                ctx.set_visuals(visuals);
            }
        }
    }

    pub fn toggle_zen_mode(&mut self) {
        self.is_zen_mode = !self.is_zen_mode;
        self.set_toast(if self.is_zen_mode {
            "Focus Mode ON (Esc or Z to exit)"
        } else {
            "Focus Mode OFF"
        });
    }

    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let wants_keyboard = ctx.egui_wants_keyboard_input();
        let mut do_escape = false;
        let mut scroll_arrow_delta = 0.0;
        let mut scroll_page_delta = 0.0;
        let mut do_home = false;
        let mut do_end = false;
        let mut do_back = false;
        let mut do_toggle_view = false;
        let mut do_toggle_width = false;
        let mut do_toggle_word_wrap = false;
        let mut do_toggle_font = false;
        let mut do_open_dialog = false;
        let mut do_cycle_theme = false;
        let mut do_toggle_zen = false;
        let mut do_toggle_help = false;
        let mut font_delta = 0.0;
        let mut font_reset = false;
        let mut do_toggle_search = false;
        let mut do_search_next = false;
        let mut do_search_prev = false;

        // 1. Collect inputs inside short ctx.input scope
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                do_escape = true;
            }

            // Ctrl+F: Open / focus search
            if i.modifiers.ctrl && i.key_pressed(Key::F) {
                do_toggle_search = true;
            }

            // F3 / Shift+F3: Next / Previous search match
            if i.key_pressed(Key::F3) {
                if i.modifiers.shift {
                    do_search_prev = true;
                } else {
                    do_search_next = true;
                }
            }

            // Keyboard document scrolling: ArrowUp / ArrowDown, PageUp / PageDown, Home / End
            if !self.show_shortcuts_modal && !wants_keyboard {
                let up_presses = if !i.modifiers.ctrl && !i.modifiers.alt {
                    i.num_presses(Key::ArrowUp)
                } else {
                    0
                };
                let down_presses = if !i.modifiers.ctrl && !i.modifiers.alt {
                    i.num_presses(Key::ArrowDown)
                } else {
                    0
                };
                if up_presses > 0 || down_presses > 0 {
                    scroll_arrow_delta = (down_presses as f32 - up_presses as f32) * 48.0;
                }

                let pgup_presses = if !i.modifiers.alt {
                    i.num_presses(Key::PageUp)
                } else {
                    0
                };
                let pgdown_presses = if !i.modifiers.alt {
                    i.num_presses(Key::PageDown)
                } else {
                    0
                };
                if pgup_presses > 0 || pgdown_presses > 0 {
                    let page_step = (self.viewport_height * 0.85).max(300.0);
                    scroll_page_delta = (pgdown_presses as f32 - pgup_presses as f32) * page_step;
                }

                if i.key_pressed(Key::Home) {
                    do_home = true;
                } else if i.key_pressed(Key::End) {
                    do_end = true;
                }
            }

            // Alt + Left or Backspace: Go back in history (Backspace only when not typing)
            if (i.modifiers.alt && i.key_pressed(Key::ArrowLeft))
                || (!wants_keyboard && i.key_pressed(Key::Backspace))
                || i.pointer.button_pressed(PointerButton::Extra1)
            {
                do_back = true;
            }

            // Ctrl+E or Tab: Toggle preview/source (Tab only when not typing)
            if (i.modifiers.ctrl && i.key_pressed(Key::E)) || (!wants_keyboard && i.key_pressed(Key::Tab)) {
                do_toggle_view = true;
            }

            // W: Toggle Width (without ctrl/alt, not while typing)
            if !wants_keyboard && !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(Key::W) {
                do_toggle_width = true;
            }

            // Alt + Z: Toggle Word Wrap
            if i.modifiers.alt && i.key_pressed(Key::Z) {
                do_toggle_word_wrap = true;
            }

            // F: Toggle Font (without ctrl/alt, not while typing)
            if !wants_keyboard && !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(Key::F) {
                do_toggle_font = true;
            }

            // Ctrl+O: Open file
            if i.modifiers.ctrl && i.key_pressed(Key::O) {
                do_open_dialog = true;
            }

            // Ctrl+T: Cycle Theme
            if i.modifiers.ctrl && i.key_pressed(Key::T) {
                do_cycle_theme = true;
            }

            // F11 or Z: Zen mode (Z only when not typing)
            if i.key_pressed(Key::F11) || (!wants_keyboard && !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(Key::Z)) {
                do_toggle_zen = true;
            }

            // ?: Shortcuts help (only when not typing)
            if !wants_keyboard && (i.key_pressed(Key::Questionmark) || (i.modifiers.shift && i.key_pressed(Key::Slash))) {
                do_toggle_help = true;
            }

            // Font Size: Ctrl + Plus / Minus / 0
            if i.modifiers.ctrl && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)) {
                font_delta = 1.0;
            } else if i.modifiers.ctrl && i.key_pressed(Key::Minus) {
                font_delta = -1.0;
            } else if i.modifiers.ctrl && i.key_pressed(Key::Num0) {
                font_reset = true;
            }
        });

        // 2. Safely execute actions OUTSIDE ctx.input lock
        if do_escape {
            if self.search_state.is_open {
                self.search_state.reset(&mut self.commonmark_cache);
            } else if self.show_shortcuts_modal {
                self.show_shortcuts_modal = false;
            } else if self.is_zen_mode {
                self.toggle_zen_mode();
            } else {
                std::process::exit(0);
            }
            return;
        }

        if do_toggle_search {
            self.search_state.is_open = true;
            self.search_state.focus_requested = true;
            if !self.search_state.query.trim().is_empty() {
                self.perform_search();
            }
        }

        if do_search_next {
            if self.search_state.is_open {
                self.navigate_search_match(1);
            } else {
                self.search_state.is_open = true;
                self.search_state.focus_requested = true;
                if !self.search_state.query.trim().is_empty() {
                    self.perform_search();
                }
            }
        }

        if do_search_prev {
            if self.search_state.is_open {
                self.navigate_search_match(-1);
            }
        }

        if scroll_arrow_delta != 0.0 {
            let target = (self.current_scroll_offset + scroll_arrow_delta).clamp(0.0, self.max_scroll_offset);
            self.pending_scroll_offset = Some(target);
        } else if scroll_page_delta != 0.0 {
            let target = (self.current_scroll_offset + scroll_page_delta).clamp(0.0, self.max_scroll_offset);
            self.pending_scroll_offset = Some(target);
        } else if do_home {
            self.pending_scroll_offset = Some(0.0);
        } else if do_end {
            self.pending_scroll_offset = Some(self.max_scroll_offset);
        }

        if do_back {
            self.go_back(ctx);
        }
        if do_toggle_view {
            if self.current_file.is_some() {
                self.toggle_view();
            }
        }
        if do_toggle_width {
            self.toggle_width();
        }
        if do_toggle_word_wrap {
            self.toggle_word_wrap();
        }
        if do_toggle_font {
            self.toggle_font();
        }
        if do_open_dialog {
            self.open_file_dialog(ctx);
        }
        if do_cycle_theme {
            self.cycle_theme(ctx);
        }
        if do_toggle_zen {
            self.toggle_zen_mode();
        }
        if do_toggle_help {
            self.show_shortcuts_modal = !self.show_shortcuts_modal;
        }
        if font_delta != 0.0 {
            self.font_size = (self.font_size + font_delta).clamp(10.0, 32.0);
            self.set_toast(format!("Font Size: {:.0}px", self.font_size));
        } else if font_reset {
            self.font_size = 16.0;
            self.set_toast("Font Size: 16px (Reset)");
        }
    }

    pub fn perform_search(&mut self) {
        let source_id = egui::Id::unique("mdv_markdown_view");
        if self.search_state.query.trim().is_empty() {
            self.search_state.total_matches = 0;
            self.search_state.active_index = 0;
            self.search_state.matches_source.clear();
            let sc = self.commonmark_cache.search_cache_mut(&source_id);
            sc.search_query.clear();
            sc.search_ranges.clear();
            sc.active_match = None;
            sc.active_search_range = None;
            return;
        }

        if let Some(ref file) = self.current_file {
            match self.view_mode {
                ViewMode::Preview => {
                    let sc = self.commonmark_cache.search_cache_mut(&source_id);
                    sc.search_query.clone_from(&self.search_state.query);
                    self.commonmark_cache.update_search_matches(&source_id, &file.content);
                    let ranges = self.commonmark_cache.search_ranges(&source_id);
                    self.search_state.total_matches = ranges.len();
                    self.search_state.active_index = self
                        .commonmark_cache
                        .active_match(&source_id)
                        .map(|idx| idx + 1)
                        .unwrap_or(if ranges.is_empty() { 0 } else { 1 });
                }
                ViewMode::Source => {
                    self.search_state.matches_source =
                        find_case_insensitive_matches(&file.content, &self.search_state.query);
                    self.search_state.total_matches = self.search_state.matches_source.len();
                    if self.search_state.total_matches > 0 {
                        self.search_state.active_index = 1;
                        self.search_state.should_scroll_source = true;
                    } else {
                        self.search_state.active_index = 0;
                    }
                }
            }
        }
    }

    pub fn navigate_search_match(&mut self, delta: isize) {
        if self.search_state.total_matches == 0 {
            return;
        }
        match self.view_mode {
            ViewMode::Preview => {
                let source_id = egui::Id::unique("mdv_markdown_view");
                self.commonmark_cache.go_to_match(&source_id, delta);
                self.search_state.active_index = self
                    .commonmark_cache
                    .active_match(&source_id)
                    .map(|idx| idx + 1)
                    .unwrap_or(0);
            }
            ViewMode::Source => {
                let len = self.search_state.total_matches as isize;
                let current_0 = (self.search_state.active_index.saturating_sub(1)) as isize;
                let next_0 = (current_0 + delta).rem_euclid(len) as usize;
                self.search_state.active_index = next_0 + 1;
                self.search_state.should_scroll_source = true;
            }
        }
    }

    fn render_search_bar(&mut self, ui: &mut Ui, _ctx: &egui::Context) {
        let mut do_nav_next = false;
        let mut do_nav_prev = false;
        let mut do_close = false;

        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(
                RichText::new(icons::MAGNIFYING_GLASS)
                    .font(FontId::new(15.0, egui::FontFamily::Name("Phosphor".into()))),
            );

            let text_edit = egui::TextEdit::singleline(&mut self.search_state.query)
                .hint_text("Find in document... (Enter/F3: next, Shift+Enter/Shift+F3: prev)")
                .desired_width(280.0)
                .id_salt("search_query_input");

            let response = ui.add(text_edit);

            if self.search_state.focus_requested {
                response.request_focus();
                self.search_state.focus_requested = false;
            }

            if response.changed() {
                self.perform_search();
            }

            if response.has_focus() {
                if ui.input(|i| i.key_pressed(Key::Enter)) {
                    if ui.input(|i| i.modifiers.shift) {
                        do_nav_prev = true;
                    } else {
                        do_nav_next = true;
                    }
                }
            }

            // Match count indicator
            if !self.search_state.query.trim().is_empty() {
                let (count_text, color) = if self.search_state.total_matches == 0 {
                    ("0/0 (No matches)".to_string(), ui.visuals().error_fg_color)
                } else {
                    (
                        format!("{}/{}", self.search_state.active_index, self.search_state.total_matches),
                        ui.visuals().text_color(),
                    )
                };
                ui.label(RichText::new(count_text).color(color).monospace().size(13.0));
            }

            ui.add_space(6.0);

            // Previous button (▲)
            let prev_btn = ui.add_enabled(
                self.search_state.total_matches > 0,
                egui::Button::new("▲").min_size(Vec2::new(24.0, 20.0)),
            ).on_hover_text("Previous match (Shift+Enter / Shift+F3)");
            if prev_btn.clicked() {
                do_nav_prev = true;
            }

            // Next button (▼)
            let next_btn = ui.add_enabled(
                self.search_state.total_matches > 0,
                egui::Button::new("▼").min_size(Vec2::new(24.0, 20.0)),
            ).on_hover_text("Next match (Enter / F3)");
            if next_btn.clicked() {
                do_nav_next = true;
            }

            // Close button (✕)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").on_hover_text("Close search (Esc)").clicked() {
                    do_close = true;
                }
            });
        });

        if do_close {
            self.search_state.reset(&mut self.commonmark_cache);
        } else if do_nav_next {
            self.navigate_search_match(1);
        } else if do_nav_prev {
            self.navigate_search_match(-1);
        }
    }

    fn render_toolbar(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

            // Left Section: Open file & document info
            if icon_button(ui, icons::FOLDER_OPEN, "Open")
                .on_hover_text("Open Markdown File (Ctrl+O)")
                .clicked()
            {
                self.open_file_dialog(ctx);
            }

            ui.separator();

            if let Some(ref file) = self.current_file {
                ui.label(
                    RichText::new(&file.filename)
                        .strong()
                        .color(ui.visuals().text_color()),
                );

                if file.path.is_some() {
                    ui.colored_label(
                        Color32::from_rgb(52, 199, 89),
                        RichText::new("● auto-reload").small(),
                    )
                    .on_hover_text("Live updates enabled on file write");
                }
            } else {
                ui.label(
                    RichText::new("No document open")
                        .italics()
                        .color(ui.visuals().weak_text_color()),
                );
            }

            // Center / Right Controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Help Button
                if icon_button(ui, icons::QUESTION, "")
                    .on_hover_text("Keyboard Shortcuts & About (?)")
                    .clicked()
                {
                    self.show_shortcuts_modal = true;
                }

                // Zen Mode Button
                let (zen_icon, zen_label) = if self.is_zen_mode {
                    (icons::CORNERS_IN, "Exit Zen")
                } else {
                    (icons::CORNERS_OUT, "Zen")
                };
                if icon_button(ui, zen_icon, zen_label)
                    .on_hover_text("Focus / Zen Mode (F11 or Z)")
                    .clicked()
                {
                    self.toggle_zen_mode();
                }

                // Theme Toggle
                let (theme_icon, theme_label) = match self.theme_mode {
                    ThemeMode::Auto => (icons::CIRCLE_HALF, "Auto"),
                    ThemeMode::Light => (icons::SUN, "Light"),
                    ThemeMode::Dark => (icons::MOON, "Dark"),
                };
                if icon_button(ui, theme_icon, theme_label)
                    .on_hover_text("Cycle Theme: Auto / Light / Dark (Ctrl+T)")
                    .clicked()
                {
                    self.cycle_theme(ctx);
                }

                // Font Toggle
                let font_label = match self.font_mode {
                    FontMode::Sans => "Source Sans 3",
                    FontMode::Mono => "JetBrains NL",
                };
                if icon_button(ui, icons::TEXT_AA, font_label)
                    .on_hover_text("Toggle Font (F): Source Sans 3 / JetBrains Mono NL")
                    .clicked()
                {
                    self.toggle_font();
                }

                // Width Toggle
                let (width_icon, width_label) = match self.width_mode {
                    WidthMode::Full => (icons::ARROWS_HORIZONTAL, "Full"),
                    WidthMode::Centered => (icons::ARROWS_IN_LINE_HORIZONTAL, "Centered"),
                };
                if icon_button(ui, width_icon, width_label)
                    .on_hover_text("Toggle Width: Full Window / Centered Column (W)")
                    .clicked()
                {
                    self.toggle_width();
                }

                // Word Wrap Toggle (for Source view)
                if self.view_mode == ViewMode::Source {
                    let wrap_text = if self.word_wrap { "Wrap: ON" } else { "Wrap: OFF" };
                    if ui
                        .selectable_label(self.word_wrap, wrap_text)
                        .on_hover_text("Toggle Word Wrap in Source View (Alt+Z)")
                        .clicked()
                    {
                        self.toggle_word_wrap();
                    }
                }

                ui.separator();

                // Segmented View Toggle: Preview vs Source
                ui.add_enabled_ui(self.current_file.is_some(), |ui| {
                    ui.horizontal(|ui| {
                        let is_preview = self.view_mode == ViewMode::Preview;
                        let is_source = self.view_mode == ViewMode::Source;

                        if ui
                            .selectable_label(is_preview, "Preview")
                            .on_hover_text("Rendered Preview (Ctrl+E or Tab)")
                            .clicked()
                            && !is_preview
                        {
                            self.toggle_view();
                        }

                        if ui
                            .selectable_label(is_source, "Source")
                            .on_hover_text("Raw Markdown Source (Ctrl+E or Tab)")
                            .clicked()
                            && !is_source
                        {
                            self.toggle_view();
                        }
                    });
                });

                ui.separator();

                // Search Toggle
                if ui
                    .selectable_label(self.search_state.is_open, "Search")
                    .on_hover_text("Find in document (Ctrl+F)")
                    .clicked()
                {
                    self.search_state.is_open = !self.search_state.is_open;
                    if self.search_state.is_open {
                        self.search_state.focus_requested = true;
                        if !self.search_state.query.trim().is_empty() {
                            self.perform_search();
                        }
                    } else {
                        self.search_state.reset(&mut self.commonmark_cache);
                    }
                }
            });
        });
    }

    fn render_shortcuts_modal(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts_modal {
            return;
        }

        let mut should_close = false;
        let mut open = true;
        egui::Window::new(format!("Keyboard Shortcuts & Info — mdv v{}", env!("CARGO_PKG_VERSION")))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_max_width(480.0);
                ui.heading("Keyboard Shortcuts");
                ui.add_space(8.0);

                egui::Grid::new("shortcuts_grid")
                    .striped(true)
                    .spacing([24.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Ctrl + E / Tab").strong().monospace());
                        ui.label("Toggle Preview / Raw Source");
                        ui.end_row();

                        ui.label(RichText::new("Ctrl + C").strong().monospace());
                        ui.label("Copy Selected Text (Source View)");
                        ui.end_row();

                        ui.label(RichText::new("Alt + Z").strong().monospace());
                        ui.label("Toggle Word Wrap (Source View)");
                        ui.end_row();

                        ui.label(RichText::new("Ctrl + F").strong().monospace());
                        ui.label("Find in Document (Preview / Source)");
                        ui.end_row();

                        ui.label(RichText::new("F3 / Shift + F3").strong().monospace());
                        ui.label("Next / Previous Search Match");
                        ui.end_row();

                        ui.label(RichText::new("W").strong().monospace());
                        ui.label("Toggle Full Width / Centered Column");
                        ui.end_row();

                        ui.label(RichText::new("F").strong().monospace());
                        ui.label("Toggle Font (Source Sans 3 / JetBrains Mono NL)");
                        ui.end_row();

                        ui.label(RichText::new("Alt + ← / Backspace").strong().monospace());
                        ui.label("Back in Document History");
                        ui.end_row();

                        ui.label(RichText::new("Ctrl + O").strong().monospace());
                        ui.label("Open Markdown File");
                        ui.end_row();

                        ui.label(RichText::new("Ctrl + T").strong().monospace());
                        ui.label("Cycle Theme (Auto / Light / Dark)");
                        ui.end_row();

                        ui.label(RichText::new("F11 / Z").strong().monospace());
                        ui.label("Toggle Zen Focus Mode");
                        ui.end_row();

                        ui.label(RichText::new("Ctrl + + / - / 0").strong().monospace());
                        ui.label("Zoom Font In / Out / Reset");
                        ui.end_row();

                        ui.label(RichText::new("↑ / ↓").strong().monospace());
                        ui.label("Scroll Up / Down");
                        ui.end_row();

                        ui.label(RichText::new("PgUp / PgDn").strong().monospace());
                        ui.label("Scroll Page Up / Down");
                        ui.end_row();

                        ui.label(RichText::new("Home / End").strong().monospace());
                        ui.label("Jump to Top / Bottom of Document");
                        ui.end_row();

                        ui.label(RichText::new("?").strong().monospace());
                        ui.label("Show this Shortcuts Help dialog");
                        ui.end_row();

                        ui.label(RichText::new("Esc").strong().monospace());
                        ui.label("Close Dialog / Exit Zen / Close mdv");
                        ui.end_row();
                    });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("mdv v{} — Pure Rust & egui", env!("CARGO_PKG_VERSION"))).small().weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if !open || should_close {
            self.show_shortcuts_modal = false;
        }
    }

    fn render_status_bar(&self, ui: &mut Ui) {
        let status_message = if let Some(ref file) = self.current_file {
            format!("Loaded {}", file.filename)
        } else {
            "Ready".to_string()
        };

        ui.horizontal(|ui| {
            let label_resp = ui.label(&status_message);
            if let Some(ref file) = self.current_file {
                if let Some(ref path) = file.path {
                    label_resp.on_hover_text(path.display().to_string());
                }
            }

            if self.search_state.is_open && !self.search_state.query.trim().is_empty() {
                ui.separator();
                let match_info = if self.search_state.total_matches == 0 {
                    "0/0".to_string()
                } else {
                    format!("{}/{}", self.search_state.active_index, self.search_state.total_matches)
                };
                ui.label(RichText::new(format!("Found: {}", match_info)).strong().monospace());
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(ref file) = self.current_file {
                    let size_str = format_file_size(file.content.len());
                    ui.label(size_str);
                    ui.separator();

                    let pct = if self.max_scroll_offset <= 0.0 {
                        100
                    } else {
                        (self.current_scroll_ratio * 100.0).round().clamp(0.0, 100.0) as u32
                    };
                    ui.label(format!("{pct}%"));
                    ui.separator();
                }
            });
        });
    }

    fn render_toast(&mut self, ctx: &egui::Context) {
        if let Some((msg, created_at)) = &self.toast {
            let elapsed = created_at.elapsed();
            if elapsed > Duration::from_secs(2) {
                self.toast = None;
                return;
            }

            // Calculate fade alpha
            let alpha = if elapsed > Duration::from_millis(1500) {
                1.0 - ((elapsed.as_millis() - 1500) as f32 / 500.0)
            } else {
                1.0
            };

            let bg_color = Color32::from_black_alpha((200.0 * alpha) as u8);
            let text_color = Color32::from_white_alpha((255.0 * alpha) as u8);

            egui::Area::new(Id::unique("toast_area"))
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -36.0])
                .order(egui::Order::Tooltip)
                .interactable(false)
                .show(ctx, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    egui::Frame::new()
                        .fill(bg_color)
                        .corner_radius(CornerRadius::same(16))
                        .inner_margin(Margin::symmetric(16, 8))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha((50.0 * alpha) as u8)))
                        .show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(msg)
                                            .font(FontId::new(14.0, egui::FontFamily::Proportional))
                                            .color(text_color),
                                    )
                                    .wrap_mode(egui::TextWrapMode::Extend),
                                );
                            });
                        });
                });

            ctx.request_repaint();
        }
    }
}

impl eframe::App for MdvApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 0. Ensure window focus on launch (especially when spawned from external file managers)
        if self.focus_frames > 0 {
            let is_focused = ctx.input(|i| i.viewport().focused == Some(true));
            if is_focused {
                self.focus_frames = 0;
            } else {
                self.focus_frames -= 1;
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }

        // 1. Process live disk updates from watcher thread
        while let Ok(update) = self.file_rx.try_recv() {
            if let Some(ref mut cur_file) = self.current_file {
                if let Some(ref cur_path) = cur_file.path {
                    if cur_path == &update.path || cur_path.file_name() == update.path.file_name() {
                        cur_file.content = update.content;
                        self.set_toast("Updated from disk");
                        ctx.request_repaint();
                    }
                }
            }
        }

        // 2. Global hotkeys
        self.handle_shortcuts(&ctx);

        // 3. Top Toolbar (reveals on hover near top edge in Zen mode)
        let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
        let near_top = mouse_pos.map(|p| p.y <= 36.0).unwrap_or(false);
        let show_toolbar = !self.is_zen_mode || near_top;

        if show_toolbar {
            let toolbar_bg = ui.visuals().window_fill();
            let toolbar_stroke = ui.visuals().widgets.noninteractive.bg_stroke.color;

            egui::Panel::top("top_toolbar")
                .frame(
                    egui::Frame::new()
                        .fill(toolbar_bg)
                        .inner_margin(Margin::symmetric(14, 8))
                        .stroke(Stroke::new(1.0, toolbar_stroke)),
                )
                .show(ui, |ui| {
                    self.render_toolbar(ui, &ctx);
                });
        }

        if self.search_state.is_open {
            let search_bar_bg = ui.visuals().window_fill();
            let search_bar_stroke = ui.visuals().widgets.noninteractive.bg_stroke.color;

            egui::Panel::top("search_bar_panel")
                .frame(
                    egui::Frame::new()
                        .fill(search_bar_bg)
                        .inner_margin(Margin::symmetric(14, 6))
                        .stroke(Stroke::new(1.0, search_bar_stroke)),
                )
                .show(ui, |ui| {
                    self.render_search_bar(ui, &ctx);
                });
        }

        // 3b. Bottom Status Bar (matches imge bottom_panel; reveals on hover near bottom edge in Zen mode)
        let screen_height = ctx.viewport_rect().height();
        let near_bottom = mouse_pos.map(|p| p.y >= screen_height - 28.0).unwrap_or(false);
        let show_status_bar = !self.is_zen_mode || near_bottom;

        if show_status_bar {
            egui::Panel::bottom("bottom_panel").show(ui, |ui| {
                self.render_status_bar(ui);
            });
        }

        // 4. Main Document Viewport
        let mut trigger_open_file = false;
        let panel_bg = ui.visuals().panel_fill;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(panel_bg))
            .show(ui, |ui| {
                // Apply dynamic font size and family
                let base = self.font_size;
                let family = if self.font_mode == FontMode::Mono {
                    egui::FontFamily::Monospace
                } else {
                    egui::FontFamily::Proportional
                };

                let mut text_styles = std::collections::BTreeMap::new();
                text_styles.insert(egui::TextStyle::Heading, egui::FontId::new(base * 1.5, family.clone()));
                text_styles.insert(egui::TextStyle::Body, egui::FontId::new(base, family.clone()));
                text_styles.insert(egui::TextStyle::Monospace, egui::FontId::new(base * 0.95, egui::FontFamily::Monospace));
                text_styles.insert(egui::TextStyle::Button, egui::FontId::new(base * 0.88, family.clone()));
                text_styles.insert(egui::TextStyle::Small, egui::FontId::new(base * 0.75, family.clone()));

                ui.style_mut().text_styles = text_styles;

                if self.font_mode == FontMode::Mono {
                    ui.style_mut().override_font_id = Some(egui::FontId::new(base, egui::FontFamily::Monospace));
                } else {
                    ui.style_mut().override_font_id = None;
                }

                if let Some(ref file) = self.current_file {
                    // Compute column width and margins
                    let available_width = ui.available_width();
                    let centered_max_width = 860.0;
                    let horizontal_margin = if self.width_mode == WidthMode::Centered {
                        ((available_width - centered_max_width) / 2.0).max(24.0)
                    } else {
                        24.0
                    };

                    // ScrollArea with relative scroll synchronization and keyboard scrolling
                    let mut scroll_area = egui::ScrollArea::vertical()
                        .auto_shrink([false, false]);

                    if let Some(target) = self.pending_scroll_offset.take() {
                        scroll_area = scroll_area.vertical_scroll_offset(target);
                    } else if let Some(ratio) = self.pending_scroll_ratio.take() {
                        let target_offset = ratio * self.max_scroll_offset;
                        scroll_area = scroll_area.vertical_scroll_offset(target_offset);
                    }

                    let line_number_color = ui.visuals().weak_text_color();
                    let mut copy_toast = false;

                    let scroll_output = scroll_area.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(horizontal_margin);

                            let content_width = if self.width_mode == WidthMode::Centered {
                                centered_max_width.min(ui.available_width() - horizontal_margin)
                            } else {
                                ui.available_width() - horizontal_margin
                            };

                            ui.allocate_ui_with_layout(
                                Vec2::new(content_width, ui.available_height()),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.add_space(16.0);

                                    match self.view_mode {
                                        ViewMode::Preview => {
                                            let mut viewer = CommonMarkViewer::new()
                                                .enable_scroll_to_heading(true);
                                            viewer = match self.theme_mode {
                                                ThemeMode::Light => viewer.syntax_theme_light("base16-ocean.light"),
                                                _ => viewer.syntax_theme_dark("base16-ocean.dark"),
                                            };
                                            let source_id = egui::Id::unique("mdv_markdown_view");
                                            viewer.show_with_id(source_id, ui, &mut self.commonmark_cache, &file.content);
                                        }
                                        ViewMode::Source => {
                                            let font_id = FontId::new(base * 0.92, egui::FontFamily::Monospace);
                                            let total_lines = file.content.lines().count().max(1);
                                            let line_num_digits = format!("{}", total_lines).len().max(3);
                                            let digit_width = ui.fonts_mut(|f| f.glyph_width(&font_id, '0')).max(base * 0.58);
                                            let gutter_width = (line_num_digits as f32 + 2.0) * digit_width;
                                            let code_width = (content_width - gutter_width).max(80.0);

                                            let max_line_len = file.content.lines().map(|l| l.chars().count()).max().unwrap_or(0);
                                            let max_text_width = (max_line_len as f32 + 4.0) * digit_width;
                                            let text_width = if self.word_wrap {
                                                code_width
                                            } else {
                                                max_text_width.max(code_width)
                                            };

                                            let start_pos = ui.cursor().min;
                                            let word_wrap = self.word_wrap;

                                            let has_search = self.search_state.is_open && !self.search_state.matches_source.is_empty();
                                            let active_idx = self.search_state.active_index;
                                            let matches_source = if has_search {
                                                self.search_state.matches_source.clone()
                                            } else {
                                                Vec::new()
                                            };

                                            let match_bg = egui::Color32::from_rgba_unmultiplied(
                                                ui.visuals().warn_fg_color.r(),
                                                ui.visuals().warn_fg_color.g(),
                                                ui.visuals().warn_fg_color.b(),
                                                90,
                                            );
                                            let active_match_bg = egui::Color32::from_rgba_unmultiplied(
                                                ui.visuals().error_fg_color.r(),
                                                ui.visuals().error_fg_color.g(),
                                                ui.visuals().error_fg_color.b(),
                                                150,
                                            );

                                            ui.horizontal_top(|ui| {
                                                ui.add_space(gutter_width);

                                                let mut content = file.content.as_str();
                                                let layouter_font_id = font_id.clone();
                                                let mut layouter = move |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                                                    let max_w = if word_wrap { wrap_width } else { f32::INFINITY };
                                                    let text = buf.as_str();
                                                    let text_color = ui.visuals().text_color();

                                                    let mut layout_job = egui::text::LayoutJob::default();
                                                    layout_job.wrap.max_width = max_w;
                                                    layout_job.keep_trailing_whitespace = true;

                                                    if matches_source.is_empty() {
                                                        layout_job.append(
                                                            text,
                                                            0.0,
                                                            egui::TextFormat::simple(layouter_font_id.clone(), text_color),
                                                        );
                                                    } else {
                                                        let mut cursor = 0;
                                                        for (i, r) in matches_source.iter().enumerate() {
                                                            let start = r.start.min(text.len());
                                                            let end = r.end.min(text.len());
                                                            if start < cursor {
                                                                continue;
                                                            }
                                                            if start > cursor {
                                                                layout_job.append(
                                                                    &text[cursor..start],
                                                                    0.0,
                                                                    egui::TextFormat::simple(layouter_font_id.clone(), text_color),
                                                                );
                                                            }
                                                            let is_active = (i + 1) == active_idx;
                                                            let bg = if is_active { active_match_bg } else { match_bg };
                                                            layout_job.append(
                                                                &text[start..end],
                                                                0.0,
                                                                egui::TextFormat {
                                                                    font_id: layouter_font_id.clone(),
                                                                    color: text_color,
                                                                    background: bg,
                                                                    ..Default::default()
                                                                },
                                                            );
                                                            cursor = end;
                                                        }
                                                        if cursor < text.len() {
                                                            layout_job.append(
                                                                &text[cursor..text.len()],
                                                                0.0,
                                                                egui::TextFormat::simple(layouter_font_id.clone(), text_color),
                                                            );
                                                        }
                                                    }

                                                    ui.fonts_mut(|f| f.layout_job(layout_job))
                                                };

                                                let output = if word_wrap {
                                                    egui::TextEdit::multiline(&mut content)
                                                        .id_salt("source_text_edit")
                                                        .font(font_id.clone())
                                                        .text_color(ui.visuals().text_color())
                                                        .desired_width(text_width)
                                                        .desired_rows(1)
                                                        .frame(egui::Frame::NONE)
                                                        .margin(egui::Margin::ZERO)
                                                        .layouter(&mut layouter)
                                                        .show(ui)
                                                } else {
                                                    egui::ScrollArea::horizontal()
                                                        .id_salt("source_hscroll")
                                                        .auto_shrink([false, false])
                                                        .show(ui, |ui| {
                                                            egui::TextEdit::multiline(&mut content)
                                                                .id_salt("source_text_edit")
                                                                .font(font_id.clone())
                                                                .text_color(ui.visuals().text_color())
                                                                .desired_width(text_width)
                                                                .desired_rows(1)
                                                                .frame(egui::Frame::NONE)
                                                                .margin(egui::Margin::ZERO)
                                                                .layouter(&mut layouter)
                                                                .show(ui)
                                                        })
                                                        .inner
                                                };

                                                if self.search_state.should_scroll_source {
                                                    if self.search_state.active_index > 0
                                                        && self.search_state.active_index <= self.search_state.matches_source.len()
                                                    {
                                                        let active_range = &self.search_state.matches_source[self.search_state.active_index - 1];
                                                        let char_count = file.content
                                                            .get(..active_range.start.min(file.content.len()))
                                                            .map_or_else(|| file.content.chars().count(), |s| s.chars().count());
                                                        let local_rect = output.galley.pos_from_cursor(egui::text::CCursor::new(char_count));
                                                        let world_rect = local_rect.translate(output.galley_pos.to_vec2());
                                                        ui.scroll_to_rect(world_rect, Some(egui::Align::Center));
                                                    }
                                                    self.search_state.should_scroll_source = false;
                                                }

                                                // Paint non-selectable line numbers aligned with each logical line
                                                let painter = ui.painter();
                                                let right_x = start_pos.x + gutter_width - (digit_width * 0.8);
                                                let mut line_num = 1;
                                                let mut is_new_line = true;

                                                for placed_row in &output.galley.rows {
                                                    if is_new_line {
                                                        let row_y = output.galley_pos.y + placed_row.pos.y;
                                                        if ui.is_rect_visible(egui::Rect::from_min_size(
                                                            egui::pos2(start_pos.x, row_y),
                                                            egui::vec2(gutter_width, placed_row.size.y),
                                                        )) {
                                                            painter.text(
                                                                egui::pos2(right_x, row_y),
                                                                egui::Align2::RIGHT_TOP,
                                                                line_num.to_string(),
                                                                font_id.clone(),
                                                                line_number_color,
                                                            );
                                                        }
                                                        line_num += 1;
                                                    }
                                                    is_new_line = placed_row.ends_with_newline;
                                                }

                                                // Handle Ctrl+C copy of selected text
                                                if let Some(cursor_range) = output.cursor_range {
                                                    if !cursor_range.is_empty() {
                                                        let copy_requested = ui.input(|i| {
                                                            (i.modifiers.command_only() && i.key_pressed(Key::C))
                                                                || i.events.iter().any(|e| matches!(e, egui::Event::Copy))
                                                        });
                                                        if copy_requested {
                                                            let selected_text = cursor_range.slice_str(&file.content);
                                                            if !selected_text.is_empty() {
                                                                ui.ctx().copy_text(selected_text.to_string());
                                                                copy_toast = true;
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    }

                                    ui.add_space(32.0);
                                },
                            );
                        });
                    });

                    if copy_toast {
                        self.set_toast("Copied selected text");
                    }

                    // Update scroll bounds and offsets
                    self.viewport_height = scroll_output.inner_rect.height();
                    let max_offset = (scroll_output.content_size.y - self.viewport_height).max(0.0);
                    self.max_scroll_offset = max_offset;
                    self.current_scroll_offset = scroll_output.state.clamped_offset().y;

                    // Calculate scroll ratio for synchronized view toggling
                    self.current_scroll_ratio = if max_offset > 0.0 {
                        (self.current_scroll_offset / max_offset).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                } else {
                    // Option A: Subtle centered prompt with an "Open File" button
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("No document open")
                                    .size(base * 1.35)
                                    .strong()
                                    .color(ui.visuals().text_color()),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Open a Markdown file to start reading")
                                    .size(base * 0.95)
                                    .color(ui.visuals().weak_text_color()),
                            );
                            ui.add_space(18.0);
                            if icon_button(ui, icons::FOLDER_OPEN, "Open File (Ctrl+O)")
                                .on_hover_text("Open Markdown File (Ctrl+O)")
                                .clicked()
                            {
                                trigger_open_file = true;
                            }
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new("Press ? for keyboard shortcuts")
                                    .size(base * 0.82)
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });
                }
            });

        if trigger_open_file {
            self.open_file_dialog(&ctx);
        }

        // 5. Intercept hyperlinks clicked in markdown preview
        let mut clicked_url = None;
        ctx.output_mut(|o| {
            o.commands.retain(|cmd| {
                if let OutputCommand::OpenUrl(open_url) = cmd {
                    clicked_url = Some(open_url.url.clone());
                    false
                } else {
                    true
                }
            });
        });
        if let Some(url) = clicked_url {
            self.handle_link_click(&url, &ctx);
        }

        // 6. Overlays: Shortcuts modal & Toast notification
        self.render_shortcuts_modal(&ctx);
        self.render_toast(&ctx);
    }
}

pub fn find_case_insensitive_matches(haystack: &str, needle: &str) -> Vec<std::ops::Range<usize>> {
    if needle.is_empty() || haystack.is_empty() {
        return Vec::new();
    }
    let needle_lower = needle.to_lowercase();
    let needle_chars: Vec<char> = needle_lower.chars().collect();
    let haystack_chars: Vec<(usize, char)> = haystack.char_indices().collect();
    let mut matches = Vec::new();

    if haystack_chars.len() < needle_chars.len() {
        return matches;
    }

    let mut i = 0;
    while i <= haystack_chars.len() - needle_chars.len() {
        let is_match = needle_chars.iter().enumerate().all(|(idx, &nc)| {
            haystack_chars[i + idx].1.to_lowercase().any(|c| c == nc)
        });
        if is_match {
            let start = haystack_chars[i].0;
            let end = if i + needle_chars.len() < haystack_chars.len() {
                haystack_chars[i + needle_chars.len()].0
            } else {
                haystack.len()
            };
            matches.push(start..end);
            i += needle_chars.len();
        } else {
            i += 1;
        }
    }
    matches
}

fn resolve_path(arg: &str) -> PathBuf {
    if let Some(stripped) = arg.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    } else if arg == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(arg)
}

fn icon_button(ui: &mut Ui, icon: &str, text: &str) -> egui::Response {
    let mut job = egui::text::LayoutJob::default();
    let text_color = ui.visuals().text_color();
    let button_font_id = egui::TextStyle::Button.resolve(ui.style());

    // Phosphor icon glyph
    job.append(
        icon,
        0.0,
        egui::TextFormat {
            font_id: FontId::new(button_font_id.size * 1.05, egui::FontFamily::Name("Phosphor".into())),
            color: text_color,
            ..Default::default()
        },
    );

    // Optional text label
    if !text.is_empty() {
        job.append(
            &format!(" {text}"),
            0.0,
            egui::TextFormat {
                font_id: button_font_id,
                color: text_color,
                ..Default::default()
            },
        );
    }

    ui.button(job)
}

pub fn format_file_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{} B", bytes)
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Register Source Sans 3 (Modern Clean Humanist Sans - High Readability)
    fonts.font_data.insert(
        "SourceSans3-Regular".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/SourceSans3-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "SourceSans3-Bold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/SourceSans3-Bold.ttf"
        ))),
    );

    // Register JetBrains Mono NL (Programmer's Font - No Ligatures)
    fonts.font_data.insert(
        "JetBrainsMonoNL-Regular".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/JetBrainsMonoNL-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "JetBrainsMonoNL-Bold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/JetBrainsMonoNL-Bold.ttf"
        ))),
    );

    // Register Phosphor Icons (Regular vector icons)
    fonts.font_data.insert(
        "Phosphor".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/Phosphor.ttf"
        ))),
    );

    // Register dedicated Phosphor FontFamily
    fonts.families.insert(
        egui::FontFamily::Name("Phosphor".into()),
        vec!["Phosphor".to_owned()],
    );

    // Source Sans 3 as primary proportional font (with Phosphor fallback for icons)
    let prop_family = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    prop_family.insert(0, "SourceSans3-Regular".to_owned());
    prop_family.push("Phosphor".to_owned());

    // JetBrains Mono NL as primary monospace font (with Phosphor fallback for icons)
    let mono_family = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    mono_family.insert(0, "JetBrainsMonoNL-Regular".to_owned());
    mono_family.push("Phosphor".to_owned());

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_mode_toggle() {
        let (file_tx, file_rx) = channel();
        let mut app = MdvApp {
            current_file: Some(FileData {
                path: None,
                filename: "test.md".to_string(),
                content: "# Hello\nWorld\n".to_string(),
            }),
            view_mode: ViewMode::Preview,
            width_mode: WidthMode::Full,
            font_mode: FontMode::Sans,
            theme_mode: ThemeMode::Auto,
            font_size: 16.0,
            word_wrap: true,
            is_zen_mode: false,
            show_shortcuts_modal: false,
            toast: None,
            focus_frames: 0,
            search_state: SearchState::default(),
            history: Vec::new(),
            current_scroll_ratio: 0.5,
            pending_scroll_ratio: None,
            current_scroll_offset: 0.0,
            max_scroll_offset: 100.0,
            viewport_height: 600.0,
            pending_scroll_offset: None,
            watcher: None,
            file_rx,
            file_tx,
            commonmark_cache: CommonMarkCache::default(),
        };

        app.toggle_view();
        assert_eq!(app.view_mode, ViewMode::Source);
        assert_eq!(app.pending_scroll_ratio, Some(0.5));

        app.toggle_view();
        assert_eq!(app.view_mode, ViewMode::Preview);
        assert_eq!(app.pending_scroll_ratio, Some(0.5));
    }

    #[test]
    fn test_selected_text_slice() {
        let text = "Line 1\nLine 2\nLine 3\n";
        let range = egui::text::CCursorRange::two(
            egui::text::CCursor::new(7),
            egui::text::CCursor::new(13),
        );
        let selected = range.slice_str(text);
        assert_eq!(selected, "Line 2");
    }

    #[test]
    fn test_word_wrap_toggle() {
        let (file_tx, file_rx) = channel();
        let mut app = MdvApp {
            current_file: None,
            view_mode: ViewMode::Source,
            width_mode: WidthMode::Full,
            font_mode: FontMode::Sans,
            theme_mode: ThemeMode::Auto,
            font_size: 16.0,
            word_wrap: true,
            is_zen_mode: false,
            show_shortcuts_modal: false,
            toast: None,
            focus_frames: 0,
            search_state: SearchState::default(),
            history: Vec::new(),
            current_scroll_ratio: 0.0,
            pending_scroll_ratio: None,
            current_scroll_offset: 0.0,
            max_scroll_offset: 0.0,
            viewport_height: 600.0,
            pending_scroll_offset: None,
            watcher: None,
            file_rx,
            file_tx,
            commonmark_cache: CommonMarkCache::default(),
        };

        assert!(app.word_wrap);
        app.toggle_word_wrap();
        assert!(!app.word_wrap);
        assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("Word Wrap: OFF"));
        app.toggle_word_wrap();
        assert!(app.word_wrap);
        assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("Word Wrap: ON"));
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(2621440), "2.5 MB");
    }

    #[test]
    fn test_handle_anchor_click() {
        let (file_tx, file_rx) = channel();
        let mut app = MdvApp {
            current_file: None,
            view_mode: ViewMode::Preview,
            width_mode: WidthMode::Full,
            font_mode: FontMode::Sans,
            theme_mode: ThemeMode::Auto,
            font_size: 16.0,
            word_wrap: true,
            is_zen_mode: false,
            show_shortcuts_modal: false,
            toast: None,
            focus_frames: 0,
            search_state: SearchState::default(),
            history: Vec::new(),
            current_scroll_ratio: 0.0,
            pending_scroll_ratio: None,
            current_scroll_offset: 0.0,
            max_scroll_offset: 0.0,
            viewport_height: 600.0,
            pending_scroll_offset: None,
            watcher: None,
            file_rx,
            file_tx,
            commonmark_cache: CommonMarkCache::default(),
        };

        let ctx = egui::Context::default();
        app.handle_link_click("#1-czym-jest-rodzina-systemow-xflow", &ctx);
        assert!(app.toast.is_none());
    }

    #[test]
    fn test_anchor_scroll_navigation() {
        let doc = r#"
# Title

[Go to section 2](#2-sekcja-druga)

## 1. Sekcja pierwsza

Tresc 1...

## 2. Sekcja druga

Tresc 2...
"#;
        let mut cache = CommonMarkCache::default();
        cache.scroll_to_heading(&Id::NULL, Some("2-sekcja-druga".to_string()));

        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                CommonMarkViewer::new()
                    .enable_scroll_to_heading(true)
                    .show(ui, &mut cache, doc);
            });
        });
        output.drop_without_applying_deltas();
    }

    #[test]
    fn test_find_case_insensitive_matches() {
        let text = "Rust is fast. Pure RUST reader. You should learn rust today.";
        let matches = find_case_insensitive_matches(text, "rust");
        assert_eq!(matches.len(), 3);
        assert_eq!(&text[matches[0].clone()], "Rust");
        assert_eq!(&text[matches[1].clone()], "RUST");
        assert_eq!(&text[matches[2].clone()], "rust");

        // Unicode / Polish letters
        let pl_text = "Żółw i ŻÓŁW idą drogą.";
        let pl_matches = find_case_insensitive_matches(pl_text, "żółw");
        assert_eq!(pl_matches.len(), 2);
        assert_eq!(&pl_text[pl_matches[0].clone()], "Żółw");
        assert_eq!(&pl_text[pl_matches[1].clone()], "ŻÓŁW");

        // Non-overlapping check
        let repeated = "aaaa";
        let rep_matches = find_case_insensitive_matches(repeated, "aa");
        assert_eq!(rep_matches.len(), 2);
        assert_eq!(rep_matches[0], 0..2);
        assert_eq!(rep_matches[1], 2..4);

        // Empty cases
        assert!(find_case_insensitive_matches(text, "").is_empty());
        assert!(find_case_insensitive_matches("", "rust").is_empty());
    }

    #[test]
    fn test_search_reset_on_view_toggle() {
        let (file_tx, file_rx) = channel();
        let mut app = MdvApp {
            current_file: Some(FileData {
                path: None,
                filename: "doc.md".to_string(),
                content: "Sample text with search target".to_string(),
            }),
            view_mode: ViewMode::Preview,
            width_mode: WidthMode::Full,
            font_mode: FontMode::Sans,
            theme_mode: ThemeMode::Auto,
            font_size: 16.0,
            word_wrap: true,
            is_zen_mode: false,
            show_shortcuts_modal: false,
            toast: None,
            focus_frames: 0,
            search_state: SearchState::default(),
            history: Vec::new(),
            current_scroll_ratio: 0.0,
            pending_scroll_ratio: None,
            current_scroll_offset: 0.0,
            max_scroll_offset: 100.0,
            viewport_height: 600.0,
            pending_scroll_offset: None,
            watcher: None,
            file_rx,
            file_tx,
            commonmark_cache: CommonMarkCache::default(),
        };

        app.search_state.is_open = true;
        app.search_state.query = "target".to_string();
        app.perform_search();
        assert_eq!(app.search_state.total_matches, 1);
        assert_eq!(app.search_state.active_index, 1);

        // Toggle to Source mode -> search must reset automatically
        app.toggle_view();
        assert_eq!(app.view_mode, ViewMode::Source);
        assert!(!app.search_state.is_open);
        assert_eq!(app.search_state.query, "");
        assert_eq!(app.search_state.total_matches, 0);
        assert_eq!(app.search_state.active_index, 0);

        // Search in Source mode
        app.search_state.is_open = true;
        app.search_state.query = "sample".to_string();
        app.perform_search();
        assert_eq!(app.search_state.total_matches, 1);
        assert_eq!(app.search_state.active_index, 1);

        // Toggle back to Preview mode -> search resets automatically again
        app.toggle_view();
        assert_eq!(app.view_mode, ViewMode::Preview);
        assert!(!app.search_state.is_open);
        assert_eq!(app.search_state.query, "");
        assert_eq!(app.search_state.total_matches, 0);
    }
}
