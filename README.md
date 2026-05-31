# gytm: TUI based Youtube Music player
Stream Youtube Music from your terminal !
# Demo 
<img width="1920" height="1080" alt="2026-05-31-215529_hyprshot" src="https://github.com/user-attachments/assets/1b512a75-ebb9-490f-94d4-b1633b9d561a" />

<br></br>
<img width="1920" height="1080" alt="2026-05-31-215546_hyprshot" src="https://github.com/user-attachments/assets/d8b68cd2-4b26-42a5-a866-505e5cfc08e6" />

<br><br>

# Features
- Stream songs on Youtube Music as saved album/playlist
- Personalized Content: Seamlessly fetch your private playlists/album using local cookie authentication.
- Search albums, save / delete albums in accounts's Library
- Stream albums from search result
## Supported OS
- **Linux** (Arch Linux)

# Build Dependencies (Only required if building from source)
- **Rust & Cargo** (1.75 or later)
- **pkg-config**
- **openssl** development headers
- **yt-dlp**: For fetching stream URLs.
- **mpv**   : The core media engine.
- **sqlite**: Local database
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
#  Authentication (Personalized Content)

`gytm` uses the `rookie` crate to automatically detect and fetch your YouTube Music session cookies from your local  web browsers. 
### Supported browsers
- Chromium based ( Brave, Google Chrome, Chromium...)
- Firefox based( Firefox, Librewolf, Zen )
# How to use:
1. Keep your Youtube/Youtube Music account signed in on your browser.
2. Launch `gytm`. It will automatically find the active session.

# ⚠️ Troubleshooting (App freezes on startup)
If you are using a standalone **Window Manager (Hyprland, i3, Sway, etc.)** and the app freezes on startup, your browser's secure storage is likely locked. Because these environments lack a default graphical interface to prompt for your password, the application hangs waiting for permission.

To fix this, you need to ensure your system's credential store is accessible before running `gytm`:

* **Option 1 (Unlock Keyring/Wallet):** Open your terminal and manually force-unlock your system's keyring or wallet daemon using its respective CLI command (e.g., `gnome-keyring-daemon --unlock` or `kwalletd6`) before launching the app.
* **Option 2 (Launch a Polkit Agent):** Ensure you have a Polkit authentication agent installed and running in your Window Manager configuration to properly handle and display graphical password prompts.

# ❤️ Credits & Inspiration
This project is inspired by: [ytermusic](https://github.com/ccgauche/ytermusic.git)
