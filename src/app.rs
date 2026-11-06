use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use egui::{
    Color32, CornerRadius, FontId, Id, Key, Margin, OutputCommand, PointerButton, RichText, Stroke, Ui, Vec2,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::icons;
use crate::watcher::{FileUpdate, FileWatcher};
use crate::welcome::WELCOME_DOC;

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

pub struct MdvApp {
    pub current_file: FileData,
    pub view_mode: ViewMode,
    pub width_mode: WidthMode,
    pub font_mode: FontMode,
    pub theme_mode: ThemeMode,
    pub font_size: f32,
    pub is_zen_mode: bool,
    pub show_shortcuts_modal: bool,
    pub toast: Option<(String, Instant)>,
    pub focus_frames: u8,

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
        // Install custom embedded typography: Inter NL (Clean Sans) and JetBrains Mono NL (Code)
        setup_custom_fonts(&cc.egui_ctx);

        // Install image loaders for egui_extras
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let (file_tx, file_rx) = channel();

        let mut app = Self {
            current_file: FileData {
                path: None,
                filename: "Welcome.md".to_string(),
                content: WELCOME_DOC.to_string(),
            },
            view_mode: ViewMode::Preview,
            width_mode: WidthMode::Full,
            font_mode: FontMode::Sans,
            theme_mode: ThemeMode::Auto,
            font_size: 16.0,
            is_zen_mode: false,
            show_shortcuts_modal: false,
            toast: None,
            focus_frames: 20,
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
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !canonical.is_file() {
            self.current_file = FileData {
                path: Some(canonical.clone()),
                filename: "File Not Found".to_string(),
                content: format!(
                    "# File Not Found\n\nThe specified file was not found:\n\n```text\n{}\n```\n\nPlease check that the file path and name are correct.",
                    canonical.display()
                ),
            };
            self.watcher = None;
            return Err(format!("File not found: {}", canonical.display()));
        }

        let content = std::fs::read_to_string(&canonical)
            .map_err(|e| format!("Failed to read '{}': {}", canonical.display(), e))?;

        let filename = canonical
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("document.md")
            .to_string();

        self.current_file = FileData {
            path: Some(canonical.clone()),
            filename: filename.clone(),
            content,
        };

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

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("{} - mdv", filename)));
        Ok(())
    }

    pub fn push_history(&mut self) {
        self.history.push(HistoryEntry {
            path: self.current_file.path.clone(),
            filename: self.current_file.filename.clone(),
            content: self.current_file.content.clone(),
            scroll_ratio: self.current_scroll_ratio,
        });
        if self.history.len() > 50 {
            self.history.remove(0);
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
                self.current_file = FileData {
                    path: None,
                    filename: prev.filename.clone(),
                    content: prev.content,
                };
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
                self.set_toast(format!("Loaded {}", self.current_file.filename));
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
            self.set_toast(format!("Anchor: {}", trimmed));
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
            .path
            .as_ref()
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
                self.set_toast(format!("Opened {}", self.current_file.filename));
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

    pub fn toggle_font(&mut self) {
        self.font_mode = match self.font_mode {
            FontMode::Sans => {
                self.set_toast("Font: JetBrains Mono NL");
                FontMode::Mono
            }
            FontMode::Mono => {
                self.set_toast("Font: Inter NL (Clean Sans)");
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
        let mut do_escape = false;
        let mut scroll_arrow_delta = 0.0;
        let mut scroll_page_delta = 0.0;
        let mut do_home = false;
        let mut do_end = false;
        let mut do_back = false;
        let mut do_toggle_view = false;
        let mut do_toggle_width = false;
        let mut do_toggle_font = false;
        let mut do_open_dialog = false;
        let mut do_cycle_theme = false;
        let mut do_toggle_zen = false;
        let mut do_toggle_help = false;
        let mut font_delta = 0.0;
        let mut font_reset = false;

        // 1. Collect inputs inside short ctx.input scope
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                do_escape = true;
            }

            // Keyboard document scrolling: ArrowUp / ArrowDown, PageUp / PageDown, Home / End
            if !self.show_shortcuts_modal {
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

            // Alt + Left or Backspace: Go back in history
            if (i.modifiers.alt && i.key_pressed(Key::ArrowLeft))
                || i.key_pressed(Key::Backspace)
                || i.pointer.button_pressed(PointerButton::Extra1)
            {
                do_back = true;
            }

            // Ctrl+E or Tab: Toggle preview/source
            if (i.modifiers.ctrl && i.key_pressed(Key::E)) || i.key_pressed(Key::Tab) {
                do_toggle_view = true;
            }

            // W: Toggle Width (without ctrl/alt)
            if !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(Key::W) {
                do_toggle_width = true;
            }

            // F: Toggle Font (without ctrl/alt)
            if !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(Key::F) {
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

            // F11 or Z: Zen mode
            if i.key_pressed(Key::F11) || (!i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(Key::Z)) {
                do_toggle_zen = true;
            }

            // ?: Shortcuts help
            if i.key_pressed(Key::Questionmark) || (i.modifiers.shift && i.key_pressed(Key::Slash)) {
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
            if self.show_shortcuts_modal {
                self.show_shortcuts_modal = false;
            } else if self.is_zen_mode {
                self.toggle_zen_mode();
            } else {
                std::process::exit(0);
            }
            return;
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
            self.toggle_view();
        }
        if do_toggle_width {
            self.toggle_width();
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

            ui.label(
                RichText::new(&self.current_file.filename)
                    .strong()
                    .color(ui.visuals().text_color()),
            );

            if self.current_file.path.is_some() {
                ui.colored_label(
                    Color32::from_rgb(52, 199, 89),
                    RichText::new("● auto-reload").small(),
                )
                .on_hover_text("Live updates enabled on file write");
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
                    FontMode::Sans => "Inter NL",
                    FontMode::Mono => "JetBrains NL",
                };
                if icon_button(ui, icons::TEXT_AA, font_label)
                    .on_hover_text("Toggle Font (F): Inter NL (Clean Sans) / JetBrains Mono NL")
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

                ui.separator();

                // Segmented View Toggle: Preview vs Source
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

                        ui.label(RichText::new("W").strong().monospace());
                        ui.label("Toggle Full Width / Centered Column");
                        ui.end_row();

                        ui.label(RichText::new("F").strong().monospace());
                        ui.label("Toggle Font (Inter NL / JetBrains Mono NL)");
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
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -28.0])
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
            if let Some(ref cur_path) = self.current_file.path {
                if cur_path == &update.path || cur_path.file_name() == update.path.file_name() {
                    self.current_file.content = update.content;
                    self.set_toast("Updated from disk");
                    ctx.request_repaint();
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

        // 4. Main Document Viewport
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
                                        let mut viewer = CommonMarkViewer::new();
                                        viewer = match self.theme_mode {
                                            ThemeMode::Light => viewer.syntax_theme_light("base16-ocean.light"),
                                            _ => viewer.syntax_theme_dark("base16-ocean.dark"),
                                        };
                                        viewer.show(ui, &mut self.commonmark_cache, &self.current_file.content);
                                    }
                                    ViewMode::Source => {
                                        let lines: Vec<&str> = self.current_file.content.lines().collect();

                                        egui::Grid::new("source_lines_grid")
                                            .num_columns(2)
                                            .spacing([14.0, 3.0])
                                            .striped(false)
                                            .show(ui, |ui| {
                                                for (idx, line) in lines.iter().enumerate() {
                                                    ui.add(
                                                        egui::Label::new(
                                                            RichText::new(format!("{:>4}", idx + 1))
                                                                .font(FontId::new(base * 0.92, egui::FontFamily::Monospace))
                                                                .color(line_number_color),
                                                        )
                                                        .selectable(false),
                                                    );

                                                    ui.add(
                                                        egui::Label::new(
                                                            RichText::new(if line.is_empty() { " " } else { line })
                                                                .font(FontId::new(base * 0.92, egui::FontFamily::Monospace)),
                                                        )
                                                        .wrap(),
                                                    );
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                }

                                ui.add_space(32.0);
                            },
                        );
                    });
                });

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
            });

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

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Register Inter NL (Modern Clean Sans - No Ligatures)
    fonts.font_data.insert(
        "InterNL-Regular".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/InterNL-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "InterNL-Bold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/InterNL-Bold.ttf"
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

    // Inter NL as primary proportional font (with Phosphor fallback for icons)
    let prop_family = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    prop_family.insert(0, "InterNL-Regular".to_owned());
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
