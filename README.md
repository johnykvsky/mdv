# mdv — Native Linux Markdown Focus Viewer

**mdv** to lekki, minimalistyczny czytnik i przeglądarka plików Markdown dla systemu Linux, stworzony z myślą o czytaniu bez rozpraszaczy (*distraction-free focus reader*). Zbudowany w języku **Rust** przy użyciu frameworka **Tauri v2** oraz natywnego linuksowego silnika **WebKitGTK**.

---

## Główne cechy

* **Pojedynczy widok z szybkim przełącznikiem**: Przełączanie między wyrenderowanym podglądem HTML a kodem źródłowym Markdown (<kbd>Ctrl</kbd> + <kbd>E</kbd> lub <kbd>Tab</kbd>) w tym samym oknie — bez dzielenia ekranu na panele boczne.
* **Synchronizacja przewijania**: Przełączając widok, zachowujesz dokładną procentową pozycję czytania.
* **Elastyczna szerokość tekstu**: Domyślnie dopasowuje się do 100% szerokości okna, z możliwością natychmiastowego przełączenia na wycentrowaną kolumnę czytelniczą (<kbd>W</kbd>).
* **Przełącznik czcionki programistycznej**: Błyskawiczne przełączanie między domyślną czcionką proporcjonalną a programistyczną **JetBrains Mono** (<kbd>F</kbd>) dla idealnego rozróżniania znaków `0`/`O` oraz `1`/`l`/`I`.
* **Historia nawigacji dokumentów**: Błyskawiczny powrót do poprzednio przeglądanych plików za pomocą <kbd>Alt</kbd> + <kbd>←</kbd>, klawisza <kbd>Backspace</kbd> lub przycisku bocznego myszy (przycisk 3).
* **Live Auto-Reload (Inotify)**: Automatyczne odświeżanie podglądu w czasie rzeczywistym po zapisaniu zmian w dowolnym edytorze (Vim, Helix, Nano, VSCode itp.).
* **Motyw jasny i ciemny**: Automatyczne dopasowanie do motywu systemowego (`auto`) lub ręczne przełączanie (<kbd>Ctrl</kbd> + <kbd>T</kbd>).
* **Tryb skupienia (Zen Mode)**: Ukrywa cały interfejs okna (<kbd>F11</kbd> lub <kbd>Z</kbd>), pozostawiając wyłącznie czysty tekst.
* **Wsparcie dla CLI**: Możliwość otwarcia pliku bezpośrednio z terminala (`mdv dokument.md`, `mdv ~/notatki.md`).
* **Zero zależności Node.js**: Interfejs to czysty, statyczny HTML/CSS/JS osadzony bezpośrednio w binarce.

---

## Skróty klawiszowe

| Skrót | Działanie |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> lub <kbd>Tab</kbd> | **Przełącz widok (Podgląd / Źródło Markdown)** |
| <kbd>W</kbd> | **Przełącz szerokość (Pełne okno / Wycentrowana kolumna)** |
| <kbd>F</kbd> | **Przełącz czcionkę (Domyślna proporcjonalna / JetBrains Mono)** |
| <kbd>Alt</kbd> + <kbd>←</kbd>, <kbd>Backspace</kbd> lub Mysz 3 | **Wróć do poprzedniego pliku (Historia dokumentów)** |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | **Przełącz motyw (Auto / Jasny / Ciemny)** |
| <kbd>F11</kbd> lub <kbd>Z</kbd> | **Tryb Zen / Pełny ekran** |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Otwórz plik przez natywne okno dialogowe |
| <kbd>Ctrl</kbd> + <kbd>+</kbd> / <kbd>-</kbd> | Powiększ / pomniejsz rozmiar czcionki |
| <kbd>Ctrl</kbd> + <kbd>0</kbd> | Zresetuj powiększenie czcionki |
| <kbd>?</kbd> | Pokaż okno pomocy ze skrótami |
| <kbd>Esc</kbd> | Wyjdź z trybu Zen / Zamknij pomoc |

---

## Wymagania systemowe i kompilacja

Aplikacja kompiluje się do małego (~10 MB) i szybkiego pliku binarnego.

### Zależności systemowe

* **Arch Linux / Manjaro:**
  ```bash
  make deps-arch
  # lub ręcznie: sudo pacman -S --needed base-devel webkit2gtk-4.1 gtk3 libsoup3 noto-fonts-emoji
  ```

* **Linux Mint / Ubuntu / Debian:**
  ```bash
  make deps-mint
  # lub ręcznie: sudo apt update && sudo apt install -y build-essential libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev fonts-noto-color-emoji
  ```

* **Obsługa kolorowych Emoji (zalecana):**
  Aplikacja posiada zintegrowany fallback czcionek dla symboli emoji w plikach Markdown. Jeśli w czytanych plikach widzisz puste prostokąty (tzw. *tofu*), zainstaluj systemowy pakiet czcionek emoji:
  - **Arch Linux / Manjaro:** `sudo pacman -S noto-fonts-emoji`
  - **Debian / Ubuntu / Mint:** `sudo apt install fonts-noto-color-emoji`

### Budowanie i zadania Makefile

W katalogu projektu dostępne są następujące polecenia:

```bash
# Wyświetlenie wszystkich dostępnych celów:
make help

# Zbudowanie zoptymalizowanej wersji release:
make build

# Uruchomienie wersji deweloperskiej z domyślnym ekranem powitalnym:
make dev

# Uruchomienie z konkretnym plikiem:
make run FILE=SAMPLE.md

# Szybka weryfikacja typów Rust (cargo check):
make check

# Uruchomienie testów Rust:
make test

# Usunięcie skompilowanych plików binarnych i artefaktów:
make clean
```

---

## Instalacja w systemie i powiązanie z plikami `.md`

Aby móc uruchamiać `mdv` z dowolnego miejsca w terminalu oraz otwierać pliki Markdown podwójnym kliknięciem w menedżerze plików, wykonaj poniższe kroki.

### Metoda 1: Automatyczna (zalecana)

W repozytorium przygotowane są gotowe reguły `Makefile`:

```bash
# 1. Kompiluje program i instaluje/aktualizuje plik wykonywalny do ~/.local/bin/mdv:
make install

# 2. Tworzy skrót na pulpicie/w menu (~/.local/share/applications), instaluje ikonę oraz rejestruje powiązania MIME:
make install-desktop

# 3. (Opcjonalnie) Samodzielne ponowne przypisanie plików .md jako domyślnych:
make associate-mime

# 4. Odinstalowanie programu, skrótu desktop oraz ikony:
make uninstall
```

---

### Metoda 2: Ręczna konfiguracja krok po kroku

Jeśli chcesz skonfigurować powiązanie ręcznie w systemie Linux:

#### Krok 1: Kopiowanie pliku wykonywalnego
Standardowym miejscem na programy użytkownika (niewymagającym uprawnień root/sudo) jest katalog `~/.local/bin`:

```bash
# Upewnij się, że katalog istnieje
mkdir -p ~/.local/bin

# Skopiuj skompilowany plik
cp src-tauri/target/release/mdv ~/.local/bin/mdv

# Nadaj uprawnienia do wykonywania
chmod +x ~/.local/bin/mdv
```

> **Wskazówka:** Upewnij się, że `~/.local/bin` znajduje się w Twojej zmiennej `$PATH` (w większości nowoczesnych dystrybucji, takich jak Manjaro czy Mint, jest to domyślnie dodane w pliku `~/.bashrc` lub `~/.zshrc`).

#### Krok 2: Utworzenie pliku aktywatora Desktop (`.desktop`)
Systemy Linux używają plików `.desktop` w katalogu `~/.local/share/applications/`, aby program był widoczny w menu startowym i w menu „Otwórz za pomocą”:

Utwórz plik `~/.local/share/applications/mdv.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=mdv
GenericName=Markdown Viewer
Comment=Minimalist Markdown Focus Reader
Exec=/home/<użytkownik>/.local/bin/mdv %U
Icon=mdv
Terminal=false
Categories=Utility;Viewer;Office;
MimeType=text/markdown;text/x-markdown;
StartupNotify=true
```

*(Zastąp `<użytkownik>` swoją nazwą użytkownika w systemie Linux)*

*Znaczenie kluczowych pól:*
* `Exec=... %U` — parametr `%U` (lub `%f`) informuje system, że kliknięty plik ma zostać przekazany jako ścieżka do naszego programu.
* `MimeType=text/markdown;text/x-markdown;` — deklaruje, że program obsługuje pliki Markdown.

Następnie odśwież bazę skrótów środowiska graficznego:
```bash
update-desktop-database ~/.local/share/applications/
```

#### Krok 3: Rejestracja `mdv` jako domyślnej aplikacji dla plików `.md`
Aby menedżer plików (Nemo w Cinnamon, Nautilus w GNOME, Thunar w XFCE) wiedział, czym otwierać pliki `.md`, powiąż typ MIME:

```bash
# Ustawienie domyślnej aplikacji dla obu wariantów MIME Markdowna
gio mime text/markdown mdv.desktop
gio mime text/x-markdown mdv.desktop

# Dodatkowo w standardzie XDG:
xdg-mime default mdv.desktop text/markdown
xdg-mime default mdv.desktop text/x-markdown
```

Sprawdzenie, czy skojarzenie działa:
```bash
gio mime text/markdown
# Powinno zwrócić: Default application for “text/markdown”: mdv.desktop
```

---

### Metoda 3: Graficzna (w menedżerze plików Nemo / Nautilus)

Możesz to również ustawić całkowicie myszką:
1. Otwórz menedżer plików i kliknij prawym przyciskiem myszy na dowolny plik z rozszerzeniem `.md`.
2. Wybierz **Właściwości** (*Properties*).
3. Przejdź do zakładki **Otwórz za pomocą** (*Open With*).
4. Z listy aplikacji wybierz **mdv**.
5. Kliknij przycisk **Ustaw jako domyślne** (*Set as default*).

Od tego momentu podwójne kliknięcie na dowolny plik `.md` natychmiast otworzy go w `mdv`.

---

## Struktura projektu

```
mdv/
├── Makefile                # Skrypty kompilacji, instalacji i powiązań MIME
├── README.md               # Podręcznik użytkownika i instrukcja konfiguracji
├── AGENTS.md               # Wytyczne techniczne dla deweloperów i agentów AI
├── SAMPLE.md               # Przykładowy dokument demonstracyjny
├── ui/                     # Statyczny interfejs czytnika
│   ├── index.html          # Struktura HTML
│   ├── style.css           # Typografia, motywy Light/Dark, tryb Zen
│   ├── app.js              # Kontroler widoków, obsługa skrótów i IPC
│   └── vendor/             # Lokalne zależności offline (marked.js, highlight.js)
└── src-tauri/              # Backend w języku Rust
    ├── Cargo.toml          # Zależności Rusta (tauri, notify, rfd, serde)
    ├── tauri.conf.json     # Konfiguracja okna i uprawnień Tauri v2
    ├── build.rs            # Skrypt budowania Tauri
    ├── capabilities/       # Zestawy uprawnień Tauri v2
    ├── icons/              # Ikony aplikacji dla systemów i menedżerów okien
    └── src/
        ├── main.rs         # Punkt wejścia i obsługa flag terminala (--help, -v)
        └── lib.rs          # Monitorowanie plików (inotify), okna dialogowe i IPC
```

---

## Licencja

Projekt udostępniony na licencji MIT / Apache 2.0.
