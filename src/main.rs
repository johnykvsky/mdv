mod app;
mod icons;
mod watcher;
mod welcome;

use std::path::PathBuf;
use app::MdvApp;

fn ensure_window_focus() {
    // Cinnamon/Muffin and X11 compositors enforce Focus Stealing Prevention (FSP)
    // when launched from file managers like Double Commander.
    // An external activation signal via xdotool (if present on X11) bypasses FSP
    // and brings the window to the active foreground.
    std::thread::spawn(|| {
        let pid = std::process::id();
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if let Ok(out) = std::process::Command::new("xdotool")
                .args(["search", "--pid", &pid.to_string(), "--onlyvisible"])
                .output()
            {
                if let Ok(stdout) = String::from_utf8(out.stdout) {
                    if let Some(win_id) = stdout.lines().next() {
                        let win_id = win_id.trim();
                        if !win_id.is_empty() {
                            let _ = std::process::Command::new("xdotool")
                                .args(["windowactivate", win_id])
                                .status();
                            break;
                        }
                    }
                }
            }
        }
    });
}

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../icons/256x256.png");
    if let Ok(image) = image::load_from_memory(icon_bytes) {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        return Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        });
    }
    None
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

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Standard CLI flags
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("mdv — Native Linux Markdown Focus Viewer\n");
        println!("Usage: mdv [OPTIONS] [PATH/TO/FILE.md]\n");
        println!("Arguments:");
        println!("  [FILE]        Markdown file to view (relative, absolute, or ~/path)");
        println!("\nOptions:");
        println!("  -h, --help    Print this help information and exit");
        println!("  -v, --version Print version information and exit\n");
        println!("Examples:");
        println!("  mdv README.md");
        println!("  mdv ~/notes/meeting.md");
        println!("  mdv /var/log/report.md");
        return Ok(());
    }

    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("mdv v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let mut target_file: Option<PathBuf> = None;
    for arg in args.into_iter().skip(1) {
        if !arg.starts_with('-') {
            target_file = Some(resolve_path(&arg));
            break;
        }
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1020.0, 720.0])
        .with_min_inner_size([500.0, 360.0])
        .with_title("mdv")
        .with_app_id("mdv")
        .with_active(true)
        .with_visible(true);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    ensure_window_focus();

    eframe::run_native(
        "mdv",
        native_options,
        Box::new(move |cc| {
            let app = MdvApp::new(cc, target_file);
            Ok(Box::new(app))
        }),
    )
}
