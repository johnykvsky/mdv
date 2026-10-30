// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
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
        return;
    }

    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("mdv v0.1.0");
        return;
    }

    mdv_lib::run();
}
