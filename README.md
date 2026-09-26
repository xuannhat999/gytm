# gytm: TUI based Youtube Music player

Stream Youtube Music from your terminal !

# Demo
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/b6cb088f-c9f4-4dc3-ab10-76432c667104" />
<br></br>
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/17d8e098-1be3-424f-b904-b5a22128d2f6" />

<br></br>

# Features

- Personalized Content: Fetch your private playlists/album using local cookie authentication.
- Interactive YTM Client Setup: Choose your browser, profile, container, and account directly from the TUI.
- Guest Mode: Use without authentication (limited to search and public content).
- Multi-account Support: Switch between multiple YouTube accounts.
- Firefox Container Support: Works with Firefox Multi-Account Containers (Gecko browsers).
- Play / Save / Remove albums in your Youtube Music Library
- Search for Albums, Songs & Videos
- Add / Remove Songs in Queue
- When select to play a song in search result, it will automatically load list of related songs into Queue
- Create / Edit personal playlists
- Keep playing in background after quit app ( Minimize )

## Supported OS

- **Linux** (Tested on Arch Linux)

# Build Dependencies (Only required if building from source)

- **Rust & Cargo** (1.85 or later)
- **pkg-config**
- **openssl** development headers
- **yt-dlp**: for fetching stream URLs.
- **mpv**   : the core media engine.
- **SQLite**: runtime dependency for reading Firefox cookie databases (via `rusqlite`).

# Optional Dependencies

- **A Nerd Font**: for icon rendering

# Installation

**- Build from source**

1. Clone this repository:

```
git clone https://github.com/xuannhat999/gytm.git
```

2. Install the binary

```
cd gytm
cargo install --bin gytm
```

**- From AUR (Arch User Repository)**  
*Using yay*

```
yay -S gytm-git
```

*Using paru*

```
paru -S gytm-git
```

# Authentication (Personalized Content)

`gytm` lets you manually select your browser, profile, and account through an interactive in-app setup flow.

### Supported browsers

| Engine | Browsers |
|--------|----------|
| **Chromium** | Chrome, Google Chrome, Brave, Brave Origin, Edge, Vivaldi |
| **Gecko** | Firefox, LibreWolf, Zen |

### Supported clients

- **Chromium-based browsers**: Cookie decryption via `rookie` (AEAD). Requires access to the browser's `Local State` file for the encryption key.
- **Gecko-based browsers**: Direct `cookies.sqlite` reading via `rusqlite`. Supports **Firefox Multi-Account Containers** for selecting specific container contexts.

### Supported features

- Multiple profile selection per browser
- Firefox Container selection (Gecko only)
- Multi-account support (`X-Goog-AuthUser` header)
- Guest mode (no authentication required)

# How to use

* **Prerequisite:** Keep your YouTube or YouTube Music account signed in on your browser.
* **First launch:** Run `gytm` in the terminal. Press `i` to open the YTM client configuration popup, then follow the setup flow:
  1. Press `b` to select your browser
  2. Press `p` to select the browser profile with your YouTube session
  3. Press `c` to select a Firefox Container (Gecko browsers only)
  4. Press `a` to fetch and select your YouTube account
* **Guest mode:** Press `i` to open the client popup, then press `g` to toggle guest mode (no authentication). In guest mode, library features are unavailable but you can still search and play public content.
* **Reload cookies:** If your YouTube session expires while `gytm` is running, press `i` then `r` to reload cookies and re-authenticate without restarting.
* **Shutdown background playback without opening app interface:** Run `gytm quit`.

# Keymap

### Global

- <kbd>Q</kbd>: Quit app
- <kbd>q</kbd>: Minimize app and keep mpv playing
- <kbd>Tab</kbd>: Switch tab
- <kbd>1</kbd>/<kbd>2</kbd>/<kbd>3</kbd>/<kbd>4</kbd>: Toggle focus area
- <kbd>i</kbd>: Open YTM client info / setup popup

### Navigation

- (<kbd>arrow up</kbd>/<kbd>k</kbd>) / (<kbd>arrow down</kbd>/<kbd>j</kbd>): Navigate up/down list items
- <kbd>l</kbd>: View Songs from album/playlist
- <kbd>h</kbd>: Close [4] Content pane 
- <kbd>4</kbd>: Focus [4] Content (only when a playlist/album is open)
- <kbd>Enter</kbd>: Play Album/Playlist/Song

### Playback

- <kbd>Space</kbd>: Pause/Resume
- <kbd>m</kbd>: Toggle shuffle On/Off
- <kbd>b</kbd>/<kbd>n</kbd>: Play previous/next song in Queue (If playmode is Shuffle, next song will be random)
- <kbd>+</kbd>/<kbd>-</kbd>: Increase/Decrease volume
- <kbd>arrow left</kbd>/<kbd>arrow right</kbd>: Go Back/Forward `seek_seconds` (default 5s, configurable in `config.toml`)

### Search

- <kbd>s</kbd>: Toggle search input (in Search Tab)
- <kbd>Esc</kbd>: Exit insert mode (in search input)
- <kbd>Enter</kbd>: Submit and search (in search input)
- <kbd>l</kbd>/<kbd>h</kbd>: Switch between Songs and Videos results (in [1] box)

### Content Management

- <kbd>x</kbd>:
  - [2]Albums Search results: Save/Unsave album
  - [1]Albums/[2]Playlists in Library: Unsave album/playlist
  - [4]Content: Save song to playlist, Unsave with <kbd>X</kbd>
- <kbd>a</kbd>:
  - [1]Songs/Videos Search results / [4]Content: Add song to Queue
  - [2]Playlist in Library: Create new playlist
- <kbd>d</kbd>:
  - [3]Queue: Remove song from Queue
- <kbd>c</kbd>: Clear Queue

### YTM Client Setup Popup (press `i`)

- <kbd>b</kbd>: Select browser
- <kbd>p</kbd>: Select profile
- <kbd>c</kbd>: Select container (Gecko browsers only)
- <kbd>a</kbd>: Fetch and select account
- <kbd>r</kbd>: Reload cookies (re-authenticate when the session expires)
- <kbd>g</kbd>: Toggle guest mode
- <kbd>Esc</kbd>: Close popup

# Configuration  
**File path:**
`$XDG_CONFIG_HOME/gytm/config.toml` (defaults to `~/.config/gytm/config.toml`)

### Default Configuration

```toml
# Theme options: "catppuccin_mocha", "tokyo_night", "gruvbox", "dracula", "nord"
theme = "catppuccin_mocha"
background = true
seek_seconds = 5
```
# ⚠️ Troubleshooting
### - App freezes on startup  
Log file path:
`$XDG_STATE_HOME/gytm/log.txt` or `~/.local/state/gytm/log.txt`  

Check the log file at `$XDG_STATE_HOME/gytm/log.txt` for specific error details.
If you are using a standalone **Window Manager (Hyprland, i3, Sway, etc.)** and the app freezes on startup, your browser's secure storage is likely locked. Because these environments lack a default graphical interface to prompt for your password, the application hangs waiting for permission.

To fix this, you need to ensure your system's credential store is accessible before running `gytm`:

- **Option 1 (Unlock Keyring/Wallet):** Open your terminal and manually force-unlock your system's keyring or wallet daemon using its respective CLI command (e.g., `gnome-keyring-daemon --unlock` or `kwalletd6`) before launching the app.
- **Option 2 (Launch a Polkit Agent):** Ensure you have a Polkit authentication agent installed and running in your Window Manager configuration to properly handle and display graphical password prompts.

### - Running in guest mode unexpectedly or request failed
- Your client state may be corrupted or browser cookies unavailable.
- Open the client config popup and reload client or re-run the setup flow.

### - Player continuously skips tracks / plays next song
- This is usually caused by YouTube updating its API or stream extraction logic, causing audio stream fetching to fail.
- **Fix:** Update `yt-dlp` to the latest version using your package manager
- If you are already on the latest stable release and the issue persists, switch to the nightly build or wait for the next update

### - Invalid cookie / Client setup fails
- Ensure the selected browser profile is currently signed into YouTube or YouTube Music.
- For Chromium browsers, ensure the browser's `Local State` file is accessible (needed for cookie decryption).
- For Gecko browsers, ensure `cookies.sqlite` exists in the profile directory and is not locked by a running browser instance.

# ❤️ Credits & Inspiration

This project is inspired by: [ytermusic](https://github.com/ccgauche/ytermusic.git)
