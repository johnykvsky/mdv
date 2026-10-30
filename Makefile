# ==============================================================================
# mdv - Native Linux Markdown Focus Viewer
# Makefile
# ==============================================================================

export PATH := $(HOME)/.cargo/bin:$(PATH)

CARGO ?= cargo
CARGO_FLAGS ?= --manifest-path src-tauri/Cargo.toml
PREFIX ?= $(HOME)/.local
BIN_DIR ?= $(PREFIX)/bin
APP_DIR ?= $(PREFIX)/share/applications
ICON_DIR ?= $(PREFIX)/share/icons/hicolor/128x128/apps
TARGET_DIR = src-tauri/target

# Colors for terminal output
CYAN  := \033[0;36m
GREEN := \033[0;32m
RESET := \033[0m

.PHONY: all help build release run dev check test clean install install-desktop associate-mime uninstall deps-arch deps-mint

all: dev

help:
	@echo "$(CYAN)mdv - Native Linux Markdown Focus Viewer$(RESET)"
	@echo ""
	@echo "Available targets:"
	@echo "  $(GREEN)make build$(RESET)           Build optimized release binary (src-tauri/target/release/mdv)"
	@echo "  $(GREEN)make run FILE=doc.md$(RESET)     Build & run with a specific file"
	@echo "  $(GREEN)make dev$(RESET)             Run debug binary with default welcome screen"
	@echo "  $(GREEN)make check$(RESET)           Run cargo check for fast compilation verification"
	@echo "  $(GREEN)make test$(RESET)            Run Rust test suite"
	@echo "  $(GREEN)make install$(RESET)         Install/update binary only to $(BIN_DIR)/mdv"
	@echo "  $(GREEN)make install-desktop$(RESET) Install desktop shortcut, icon & file associations"
	@echo "  $(GREEN)make associate-mime$(RESET)  Set mdv as default application for .md files"
	@echo "  $(GREEN)make uninstall$(RESET)       Remove binary, desktop file and icon"
	@echo "  $(GREEN)make clean$(RESET)           Clean cargo build artifacts"
	@echo "  $(GREEN)make deps-arch$(RESET)       Show/install Arch/Manjaro system dependencies"
	@echo "  $(GREEN)make deps-mint$(RESET)       Show/install Linux Mint/Ubuntu system dependencies"

# Build optimized release binary
build: release

release:
	@echo "$(CYAN)Building release binary...$(RESET)"
	$(CARGO) build --release $(CARGO_FLAGS)
	@echo "$(GREEN)Release build complete: $(TARGET_DIR)/release/mdv$(RESET)"

# Build and run with optional file argument: make run FILE=SAMPLE.md
run:
	$(CARGO) run $(CARGO_FLAGS) -- $(FILE)

# Dev mode without arguments (shows welcome / guide)
dev:
	$(CARGO) run $(CARGO_FLAGS)

# Fast type check
check:
	$(CARGO) check $(CARGO_FLAGS)

# Test suite
test:
	$(CARGO) test $(CARGO_FLAGS)

# Clean cargo cache
clean:
	$(CARGO) clean $(CARGO_FLAGS)

# Install/update only the binary into user environment (~/.local/bin)
install: release
	@mkdir -p $(BIN_DIR)
	@install -m 755 $(TARGET_DIR)/release/mdv $(BIN_DIR)/mdv
	@echo "$(GREEN)Installed mdv binary to $(BIN_DIR)/mdv$(RESET)"

# Setup desktop shortcut (.desktop), application icon, and file associations
install-desktop:
	@mkdir -p $(APP_DIR) $(ICON_DIR)
	@install -m 644 src-tauri/icons/128x128.png $(ICON_DIR)/mdv.png
	@printf "[Desktop Entry]\nType=Application\nName=mdv\nGenericName=Markdown Viewer\nComment=Minimalist Markdown Focus Reader\nExec=$(BIN_DIR)/mdv %%U\nIcon=mdv\nTerminal=false\nCategories=Utility;Viewer;Office;\nMimeType=text/markdown;text/x-markdown;\nStartupNotify=true\n" > $(APP_DIR)/mdv.desktop
	@update-desktop-database $(APP_DIR) 2>/dev/null || true
	@xdg-mime default mdv.desktop text/markdown 2>/dev/null || true
	@xdg-mime default mdv.desktop text/x-markdown 2>/dev/null || true
	@gio mime text/markdown mdv.desktop 2>/dev/null || true
	@gio mime text/x-markdown mdv.desktop 2>/dev/null || true
	@echo "$(GREEN)Created desktop shortcut: $(APP_DIR)/mdv.desktop$(RESET)"
	@echo "$(GREEN)Installed icon: $(ICON_DIR)/mdv.png$(RESET)"
	@echo "$(GREEN)Associated .md files with mdv$(RESET)"

# Set mdv as default app for markdown files
associate-mime:
	@xdg-mime default mdv.desktop text/markdown
	@xdg-mime default mdv.desktop text/x-markdown
	@gio mime text/markdown mdv.desktop 2>/dev/null || true
	@gio mime text/x-markdown mdv.desktop 2>/dev/null || true
	@echo "$(GREEN)mdv is now set as the default viewer for Markdown files (.md)$(RESET)"

uninstall:
	@rm -f $(BIN_DIR)/mdv
	@rm -f $(APP_DIR)/mdv.desktop
	@rm -f $(ICON_DIR)/mdv.png
	@update-desktop-database $(APP_DIR) 2>/dev/null || true
	@echo "Removed mdv binary, desktop entry, and icon."

# Dependency helpers for distros
deps-arch:
	@echo "Arch / Manjaro dependencies:"
	sudo pacman -S --needed base-devel webkit2gtk-4.1 gtk3 libsoup3 noto-fonts-emoji

deps-mint:
	@echo "Linux Mint / Ubuntu LTS dependencies:"
	sudo apt update && sudo apt install -y build-essential libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev fonts-noto-color-emoji
