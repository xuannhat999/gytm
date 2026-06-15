# gytm: TUI based Youtube Music player

Stream Youtube Music from your terminal !

# Demo

<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/f4fad48d-8a48-4bcd-95f7-0c066c8e3e93" />
<br></br>
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/398677ae-d35f-406c-adf2-50dca21952d6" />

<https://github.com/user-attachments/assets/b9e58baa-c24d-47c4-826a-e26c765b0905>

<br><br>

# Features

- Personalized Content: Seamlessly fetch your private playlists/album using local cookie authentication.
- Play / Save / Remove albums in your Youtube Music Library
- Search for Albums & Songs
- Add / Remove Songs in Queue
- When select to play a song in search result, it will automatically load list of related songs into Queue
- Create / Edit personal playlists

## Supported OS

- **Linux** (Tested on Arch Linux)

# Build Dependencies (Only required if building from source)

- **Rust & Cargo** (1.85 or later)
- **pkg-config**
- **openssl** development headers
- **yt-dlp**: for fetching stream URLs.
- **mpv**   : the core media engine.
- **sqlite**: local database

# Optional Dependencies

- **A Nerd Font**: for icon rendering

# Installation

**- Build from source**

1. Clone this repository:

```
git clone https://github.com/xuannhat999/gytm.git
```

1. Install the binary

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

`gytm` uses the `rookie` crate to automatically detect and fetch your YouTube Music session cookies from your local  web browsers.

### Supported browsers

- Chromium based ( Brave, Google Chrome, Chromium...)
- Firefox based( Firefox, Librewolf, Zen )

# How to use

1. Keep your Youtube/Youtube Music account signed in on your browser.
2. Launch `gytm`. It will automatically find the active session.

# Keymap

- <kbd>q</kbd>: Quit app
- <kbd>Tab</kbd>: Switch tab
- <kbd>1</kbd>/<kbd>2</kbd>/<kbd>3</kbd>/<kbd>4</kbd>: Toggle focus area
- (<kbd>arrow up</kbd>/<kbd>k</kbd>) / (<kbd>arrow down</kbd>/<kbd>j</kbd>): Navigate up/down list items
- <kbd>l</kbd>: View Songs from album/playlist ( In Library )
- <kbd>Enter</kbd>: Play Album/Playlist/Song
- <kbd>Space</kbd>: Pause/Resume
- <kbd>m</kbd>: Toggle playmode (Default/Shuffle)
- <kbd>b</kbd>/<kbd>n</kbd>: Play previous/next song in Queue ( If playmode is Shuffle, next song will be random )
- <kbd>+</kbd>/<kbd>-</kbd>: Increase/Decrease volume
- <kbd>arrow left</kbd>/<kbd>arrow right</kbd>: Go Back/Forward 5s
- <kbd>s</kbd>: Toggle search input ( in Search Tab )
- <kbd>Esc</kbd>: Exit insert mode ( in search input )
- <kbd>Enter</kbd>: Submit and search ( in search input )
- <kbd>x</kbd>:
  - [1]Albums Search results: Save/Unsave album
  - [1]Albums/[2]Playlists in Library: Unsave album/playlist
  - [4]Songs in Library: Remove song from playlist
- <kbd>a</kbd>:
  - [2]Songs Search results  / [4]Songs in Library: Add song to Queue
  - [2]Playlist in Library: Create new playlist
- <kbd>d</kbd>:
  - [3]Queue: Remove song from Queue
- <kbd>c</kbd>: Clear Queue

# ⚠️ Troubleshooting (App freezes on startup)

If you are using a standalone **Window Manager (Hyprland, i3, Sway, etc.)** and the app freezes on startup, your browser's secure storage is likely locked. Because these environments lack a default graphical interface to prompt for your password, the application hangs waiting for permission.

To fix this, you need to ensure your system's credential store is accessible before running `gytm`:

- **Option 1 (Unlock Keyring/Wallet):** Open your terminal and manually force-unlock your system's keyring or wallet daemon using its respective CLI command (e.g., `gnome-keyring-daemon --unlock` or `kwalletd6`) before launching the app.
- **Option 2 (Launch a Polkit Agent):** Ensure you have a Polkit authentication agent installed and running in your Window Manager configuration to properly handle and display graphical password prompts.

# ❤️ Credits & Inspiration

This project is inspired by: [ytermusic](https://github.com/ccgauche/ytermusic.git)
