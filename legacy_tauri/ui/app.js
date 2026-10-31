// ============================================================
// mdv - Minimalist Markdown Focus Viewer (Frontend Controller)
// ============================================================

(function () {
  'use strict';

  // State
  let currentFile = null;
  let currentRawMarkdown = '';
  let currentView = 'preview'; // 'preview' | 'source'
  let currentTheme = localStorage.getItem('mdv_theme') || 'auto';
  let currentWidth = localStorage.getItem('mdv_width') || 'full'; // 'full' | 'centered'
  let currentFont = localStorage.getItem('mdv_font') || 'sans'; // 'sans' | 'mono'
  let fontSizeZoom = 16;
  let isZenMode = false;

  // DOM Elements
  const body = document.body;
  const viewport = document.getElementById('viewport');
  const previewContainer = document.getElementById('preview-container');
  const sourceContainer = document.getElementById('source-container');
  const markdownBody = document.getElementById('markdown-body');
  const sourceCodeEl = document.getElementById('source-code');
  const docTitle = document.getElementById('doc-title');
  const fileBadge = document.getElementById('file-badge');
  const viewPreviewBtn = document.getElementById('view-preview-btn');
  const viewSourceBtn = document.getElementById('view-source-btn');
  const widthBtn = document.getElementById('width-btn');
  const fontBtn = document.getElementById('font-btn');
  const themeBtn = document.getElementById('theme-btn');
  const themeIconSun = document.getElementById('theme-icon-sun');
  const themeIconMoon = document.getElementById('theme-icon-moon');
  const zenBtn = document.getElementById('zen-btn');
  const helpBtn = document.getElementById('help-btn');
  const btnOpen = document.getElementById('btn-open');
  const modalShortcuts = document.getElementById('modal-shortcuts');
  const btnCloseModal = document.getElementById('btn-close-modal');
  const toastEl = document.getElementById('toast');

  // Tauri IPC helper
  const tauri = window.__TAURI__ ? window.__TAURI__.core : null;
  const tauriEvent = window.__TAURI__ ? window.__TAURI__.event : null;

  // Configure Marked
  if (typeof marked !== 'undefined') {
    marked.setOptions({
      gfm: true,
      breaks: true,
      pedantic: false,
    });
  }

  // Toast notification
  let toastTimer = null;
  function showToast(message, durationMs = 2000) {
    if (!toastEl) return;
    toastEl.textContent = message;
    toastEl.classList.remove('hidden');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastEl.classList.add('hidden');
    }, durationMs);
  }

  // HTML escape helper
  function escapeHtml(text) {
    if (!text) return '';
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

  // Render Markdown & update Source
  function renderDocument(content, filename = 'document.md', filePath = '') {
    currentRawMarkdown = content;

    // 1. Render Preview
    if (typeof marked !== 'undefined') {
      markdownBody.innerHTML = marked.parse(content);

      // Syntax highlight code blocks
      if (typeof hljs !== 'undefined') {
        markdownBody.querySelectorAll('pre code').forEach((block) => {
          hljs.highlightElement(block);
        });
      }
    } else {
      markdownBody.textContent = content;
    }

    // 2. Render Source (Line by line with preserved alignment & wrapping)
    const lines = content.split('\n');
    const sourceHtml = lines.map((line, idx) => {
      const escaped = escapeHtml(line);
      const textDisplay = escaped.length > 0 ? escaped : '&nbsp;';
      return `<div class="source-line"><span class="line-no">${idx + 1}</span><span class="line-text">${textDisplay}</span></div>`;
    }).join('');

    sourceCodeEl.innerHTML = sourceHtml;

    // 3. Update Title & Badges
    docTitle.textContent = filename;
    docTitle.parentElement.title = filePath || filename;
    if (filePath) {
      fileBadge.classList.remove('hidden');
    }

    // 4. Update Window Title if in Tauri
    if (tauri) {
      tauri.invoke('set_window_title', { title: `${filename} - mdv` }).catch(() => {});
    } else {
      document.title = `${filename} - mdv`;
    }
  }

  // View Switcher (Preview vs Source) with synchronized scroll ratio
  function setView(view) {
    if (view === currentView) return;

    // Preserve scroll percentage
    const maxScroll = viewport.scrollHeight - viewport.clientHeight;
    const scrollRatio = maxScroll > 0 ? viewport.scrollTop / maxScroll : 0;

    currentView = view;
    body.setAttribute('data-view', view);

    if (view === 'preview') {
      previewContainer.classList.add('active-view');
      sourceContainer.classList.remove('active-view');
      viewPreviewBtn.classList.add('active');
      viewSourceBtn.classList.remove('active');
    } else {
      previewContainer.classList.remove('active-view');
      sourceContainer.classList.add('active-view');
      viewPreviewBtn.classList.remove('active');
      viewSourceBtn.classList.add('active');
    }

    // Restore scroll position
    requestAnimationFrame(() => {
      const newMaxScroll = viewport.scrollHeight - viewport.clientHeight;
      if (newMaxScroll > 0) {
        viewport.scrollTop = scrollRatio * newMaxScroll;
      }
    });
  }

  function toggleView() {
    setView(currentView === 'preview' ? 'source' : 'preview');
  }

  // Width Management (Full width vs Centered Column)
  function applyWidth(width) {
    currentWidth = width;
    localStorage.setItem('mdv_width', width);
    body.setAttribute('data-width', width);
    widthBtn.title = width === 'full' 
      ? 'Current: Full Width (Press W to switch to Centered Column)' 
      : 'Current: Centered Column (Press W to switch to Full Width)';
  }

  function toggleWidth() {
    const nextWidth = currentWidth === 'full' ? 'centered' : 'full';
    applyWidth(nextWidth);
    showToast(nextWidth === 'full' ? 'Width: Full Window' : 'Width: Centered Column');
  }

  // Font Management (Default Sans vs JetBrains Mono / Coding Font)
  function applyFont(font) {
    currentFont = font;
    localStorage.setItem('mdv_font', font);
    body.setAttribute('data-font', font);
    if (fontBtn) {
      const isMono = font === 'mono';
      fontBtn.title = isMono
        ? 'Current Font: JetBrains Mono (Press F to switch to Default Sans)'
        : 'Current Font: Default Sans (Press F to switch to JetBrains Mono)';
      fontBtn.classList.toggle('active', isMono);
    }
  }

  function toggleFont() {
    const nextFont = currentFont === 'mono' ? 'sans' : 'mono';
    applyFont(nextFont);
    showToast(nextFont === 'mono' ? 'Font: JetBrains Mono (Code)' : 'Font: Default Sans');
  }

  // Theme Management
  const isSystemDark = () => window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;

  function applyTheme(theme) {
    currentTheme = theme;
    localStorage.setItem('mdv_theme', theme);
    document.documentElement.setAttribute('data-theme', theme);
    body.setAttribute('data-theme', theme);

    const effectiveDark = theme === 'dark' || (theme === 'auto' && isSystemDark());

    if (effectiveDark) {
      themeIconSun.classList.add('hidden');
      themeIconMoon.classList.remove('hidden');
    } else {
      themeIconSun.classList.remove('hidden');
      themeIconMoon.classList.add('hidden');
    }

    if (theme === 'dark') {
      themeBtn.title = 'Current Theme: Dark (Ctrl+T to switch)';
    } else if (theme === 'light') {
      themeBtn.title = 'Current Theme: Light (Ctrl+T to switch)';
    } else {
      themeBtn.title = `Current Theme: System Auto (${effectiveDark ? 'Dark' : 'Light'}) (Ctrl+T to switch)`;
    }
  }

  // React live to OS dark/light mode changes when in 'auto' mode
  if (window.matchMedia) {
    try {
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (currentTheme === 'auto') {
          applyTheme('auto');
        }
      });
    } catch (_) {}
  }

  function cycleTheme() {
    if (currentTheme === 'auto') {
      applyTheme('light');
      showToast('Theme: Light');
    } else if (currentTheme === 'light') {
      applyTheme('dark');
      showToast('Theme: Dark');
    } else {
      applyTheme('auto');
      showToast('Theme: System Auto');
    }
  }

  // Focus / Zen Mode
  function toggleZenMode() {
    isZenMode = !isZenMode;
    body.classList.toggle('zen-mode', isZenMode);
    showToast(isZenMode ? 'Focus Mode ON (Esc to exit)' : 'Focus Mode OFF');
  }

  // Font Zoom
  function adjustZoom(delta) {
    fontSizeZoom = Math.min(28, Math.max(12, fontSizeZoom + delta));
    document.documentElement.style.setProperty('--font-size-base', `${fontSizeZoom}px`);
  }

  function resetZoom() {
    fontSizeZoom = 16;
    document.documentElement.style.setProperty('--font-size-base', '16px');
  }

  // Navigation History
  const navHistory = [];

  function pushHistory(file) {
    if (file && file.path) {
      navHistory.push({
        path: file.path,
        filename: file.filename,
        content: file.content,
        scrollTop: viewport.scrollTop,
      });
      if (navHistory.length > 50) navHistory.shift();
    }
  }

  async function goBack() {
    if (navHistory.length === 0) {
      showToast('No previous document in history');
      return false;
    }
    const prev = navHistory.pop();
    if (prev && prev.path) {
      if (tauri) {
        try {
          const fileData = await tauri.invoke('open_file_path', { path: prev.path });
          if (fileData) {
            currentFile = fileData;
            renderDocument(fileData.content, fileData.filename, fileData.path);
            if (prev.scrollTop) {
              requestAnimationFrame(() => {
                viewport.scrollTop = prev.scrollTop;
              });
            }
            showToast(`Returned to ${fileData.filename}`);
            return true;
          }
        } catch (e) {
          showToast(`Error: ${e}`);
        }
      }
    }
    return false;
  }

  async function handleLinkClick(href) {
    if (!href) return;

    // 1. In-document anchor link (e.g. #installation or #section-1)
    if (href.startsWith('#')) {
      const anchorId = decodeURIComponent(href.slice(1));
      const targetEl = document.getElementById(anchorId) || document.querySelector(`[name="${anchorId}"]`);
      if (targetEl) {
        targetEl.scrollIntoView({ behavior: 'smooth' });
      } else {
        showToast(`Anchor #${anchorId} not found`);
      }
      return;
    }

    if (!tauri) {
      // Standalone web browser fallback
      if (href.startsWith('http://') || href.startsWith('https://')) {
        window.open(href, '_blank', 'noopener,noreferrer');
      } else {
        showToast('Local file links require running in the desktop app.');
      }
      return;
    }

    // Save previous document for history navigation before switching
    if (currentFile && currentFile.path) {
      pushHistory(currentFile);
    }

    try {
      const result = await tauri.invoke('open_link', { link: href });
      if (result) {
        // Markdown file opened in mdv
        currentFile = result;
        renderDocument(result.content, result.filename, result.path);

        // Check if link had an in-file anchor (e.g. file.md#section)
        const hashIdx = href.indexOf('#');
        if (hashIdx !== -1) {
          const targetAnchor = decodeURIComponent(href.slice(hashIdx + 1));
          setTimeout(() => {
            const targetEl = document.getElementById(targetAnchor) || document.querySelector(`[name="${targetAnchor}"]`);
            if (targetEl) targetEl.scrollIntoView({ behavior: 'smooth' });
          }, 80);
        }

        showToast(`Opened ${result.filename}`);
      } else {
        // External link or non-markdown file opened in default system application
        showToast('Opened in default application');
      }
    } catch (err) {
      console.error('Failed to open link:', err);
      // Remove failed navigation from history
      if (navHistory.length > 0) navHistory.pop();
      showToast(`Error: ${err}`, 3500);
    }
  }

  // Open file dialog via Tauri
  async function openFile() {
    if (!tauri) {
      showToast('File picker requires running inside the mdv desktop app.');
      return;
    }
    try {
      const fileData = await tauri.invoke('open_file_dialog');
      if (fileData) {
        currentFile = fileData;
        renderDocument(fileData.content, fileData.filename, fileData.path);
        showToast(`Loaded ${fileData.filename}`);
      }
    } catch (err) {
      console.error('Error opening file:', err);
      showToast(`Error: ${err}`);
    }
  }

  // Welcome Markdown
  const WELCOME_DOC = `# Welcome to mdv v0.1.0

**mdv** is a native, distraction-free **Markdown Focus Viewer** for Linux.

---

### Key Features

* **Instant View Toggle**: Switch between **Rendered Preview** and **Raw Source** in the same window (no distracting side-by-side panels).
* **Flexible Width**: Automatically adapts to full window width or centers into a clean reading column (press <kbd>W</kbd>).
* **Live Auto-Reload**: Whenever you save your markdown file in Vim, Helix, or any editor, mdv instantly updates on screen.
* **Smart File Links**: Click local markdown links, images, PDFs, or web URLs with built-in navigation history (<kbd>Alt</kbd> + <kbd>←</kbd>).
* **Zen Focus Mode**: Strip away all window chrome and focus purely on reading.
* **Light & Dark Themes**: System-adaptive with custom handcrafted contrast palettes.

---

### Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> or <kbd>Tab</kbd> | **Toggle Preview / Source** |
| <kbd>W</kbd> | **Toggle Full Width / Centered Column** |
| <kbd>F</kbd> | **Toggle Font (Default Sans / JetBrains Mono)** |
| <kbd>Alt</kbd> + <kbd>←</kbd> or <kbd>Backspace</kbd> | **Back in Document History** |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | **Open Markdown File** |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | **Cycle Theme (Auto / Light / Dark)** |
| <kbd>F11</kbd> or <kbd>Z</kbd> | **Toggle Zen Focus Mode** |
| <kbd>Ctrl</kbd> + <kbd>+</kbd> / <kbd>-</kbd> | Zoom Font In / Out |
| <kbd>Ctrl</kbd> + <kbd>0</kbd> | Reset Font Size |
| <kbd>?</kbd> | Show Keyboard Shortcuts & About |
| <kbd>Esc</kbd> | Exit Focus Mode / Close Dialog |

---

### Code Syntax Example

\`\`\`rust
fn main() {
    let focus = "100%";
    println!("Reading distraction-free with {} focus!", focus);
}
\`\`\`

> *"Simplicity is the ultimate sophistication."* — Leonardo da Vinci

*Tip: You can launch from the terminal with \`mdv my-document.md\` or click **Open** above!*
`;

  // Event Listeners
  viewPreviewBtn.addEventListener('click', () => setView('preview'));
  viewSourceBtn.addEventListener('click', () => setView('source'));
  widthBtn.addEventListener('click', toggleWidth);
  if (fontBtn) fontBtn.addEventListener('click', toggleFont);
  themeBtn.addEventListener('click', cycleTheme);
  zenBtn.addEventListener('click', toggleZenMode);
  btnOpen.addEventListener('click', openFile);

  // Link click interception for local files & external links
  markdownBody.addEventListener('click', (e) => {
    const link = e.target.closest('a');
    if (!link) return;
    const href = link.getAttribute('href');
    if (!href) return;
    e.preventDefault();
    handleLinkClick(href);
  });

  // Mouse back button (button 3 / thumb back)
  window.addEventListener('mouseup', (e) => {
    if (e.button === 3) {
      e.preventDefault();
      goBack();
    }
  });

  helpBtn.addEventListener('click', () => modalShortcuts.classList.remove('hidden'));
  btnCloseModal.addEventListener('click', () => modalShortcuts.classList.add('hidden'));
  modalShortcuts.addEventListener('click', (e) => {
    if (e.target === modalShortcuts) modalShortcuts.classList.add('hidden');
  });

  // Reveal toolbar on mouse hover near top in Zen Mode
  document.addEventListener('mousemove', (e) => {
    if (isZenMode) {
      if (e.clientY <= 25) {
        body.classList.add('zen-hover');
      } else if (e.clientY > 60) {
        body.classList.remove('zen-hover');
      }
    }
  });

  // Global Keyboard Shortcuts
  window.addEventListener('keydown', (e) => {
    // Esc: Close modal or exit zen mode
    if (e.key === 'Escape') {
      if (!modalShortcuts.classList.contains('hidden')) {
        modalShortcuts.classList.add('hidden');
        return;
      }
      if (isZenMode) {
        toggleZenMode();
        return;
      }
    }

    // Alt + Left or Backspace: Go back in document history
    if ((e.altKey && e.key === 'ArrowLeft') || (e.key === 'Backspace' && document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA')) {
      if (navHistory.length > 0) {
        e.preventDefault();
        goBack();
        return;
      }
    }

    // Ctrl+E or Tab: Toggle preview/source
    if ((e.ctrlKey && (e.key === 'e' || e.key === 'E')) || (e.key === 'Tab' && !e.ctrlKey && !e.altKey && !e.metaKey)) {
      e.preventDefault();
      toggleView();
      return;
    }

    // W: Toggle Width
    if (!e.ctrlKey && !e.altKey && (e.key === 'w' || e.key === 'W')) {
      if (document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
        e.preventDefault();
        toggleWidth();
        return;
      }
    }

    // F: Toggle Font (Default Sans vs JetBrains Mono)
    if (!e.ctrlKey && !e.altKey && (e.key === 'f' || e.key === 'F')) {
      if (document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
        e.preventDefault();
        toggleFont();
        return;
      }
    }

    // Ctrl+O: Open file
    if (e.ctrlKey && (e.key === 'o' || e.key === 'O')) {
      e.preventDefault();
      openFile();
      return;
    }

    // Ctrl+T: Cycle Theme
    if (e.ctrlKey && (e.key === 't' || e.key === 'T')) {
      e.preventDefault();
      cycleTheme();
      return;
    }

    // F11 or Z (without ctrl): Zen mode
    if (e.key === 'F11' || (!e.ctrlKey && !e.altKey && (e.key === 'z' || e.key === 'Z'))) {
      if (document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
        e.preventDefault();
        toggleZenMode();
        return;
      }
    }

    // Shortcuts help: ?
    if (e.key === '?' && !e.ctrlKey && !e.altKey) {
      if (document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
        e.preventDefault();
        modalShortcuts.classList.toggle('hidden');
        return;
      }
    }

    // Font Zoom: Ctrl + Plus / Minus / 0
    if (e.ctrlKey && (e.key === '=' || e.key === '+')) {
      e.preventDefault();
      adjustZoom(1);
    } else if (e.ctrlKey && (e.key === '-' || e.key === '_')) {
      e.preventDefault();
      adjustZoom(-1);
    } else if (e.ctrlKey && e.key === '0') {
      e.preventDefault();
      resetZoom();
    }
  });

  // Initialization
  async function init() {
    applyTheme(currentTheme);
    applyWidth(currentWidth);
    applyFont(currentFont);

    if (tauri) {
      // 0. Fetch version from backend
      try {
        const ver = await tauri.invoke('get_app_version');
        if (ver) {
          const badge = document.getElementById('app-version-badge');
          const footerVer = document.getElementById('app-version-text');
          if (badge) badge.textContent = ver;
          if (footerVer) footerVer.textContent = ver;
          if (helpBtn) helpBtn.title = `Shortcuts & Info — mdv ${ver}`;
        }
      } catch (_) {}

      // 1. Check for initial file passed via command line
      try {
        const initialFile = await tauri.invoke('get_initial_file');
        if (initialFile) {
          currentFile = initialFile;
          renderDocument(initialFile.content, initialFile.filename, initialFile.path);
        } else {
          renderDocument(WELCOME_DOC, 'Welcome.md', '');
        }
      } catch (e) {
        console.error('Failed to get initial file:', e);
        renderDocument(WELCOME_DOC, 'Welcome.md', '');
      }

      // 2. Listen for live-reload updates from Rust file watcher
      if (tauriEvent) {
        try {
          await tauriEvent.listen('file-updated', (event) => {
            const data = event.payload;
            if (data && data.content) {
              renderDocument(data.content, data.filename, data.path);
              showToast('Updated from disk');
            }
          });
        } catch (e) {
          console.error('Failed to register file-updated listener:', e);
        }
      }
    } else {
      // Running standalone in browser
      renderDocument(WELCOME_DOC, 'Welcome.md', '');
    }
  }

  // Start app
  init();
})();
