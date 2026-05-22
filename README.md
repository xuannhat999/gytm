# gytm: TUI based Youtube Music player
Stream Youtube Music from your terminal !
# Demo 
<img width="1920" height="1080" alt="2026-05-03-191945_hyprshot" src="https://github.com/user-attachments/assets/11c058db-83f5-40ca-95ea-cb2ac19d1f0d" />
<br><br>
<img width="1920" height="1080" alt="2026-05-03-192008_hyprshot" src="https://github.com/user-attachments/assets/cb056fea-c925-4a18-9c0e-eb5f6a8a622c" />

# Features
- Stream songs on Youtube Music as saved album/playlist
- Personalized Content: Seamlessly fetch your private playlists/album using local cookie authentication.

#  Packages required
- **yt-dlp**: For fetching stream URLs.
- **mpv**   : The core media engine.
- **socat** : For IPC communication with mpv.
- **Rust**  : To build the project from source.
## Supported OS
- **Linux** (Tested on **Arch Linux** with **Hyprland**)
# Installation
1. Clone this repository:
```
git clone git@github.com:xuannhat999/gytm.git
```
2. Install the binary
```
cd  gytm
cargo install --path crates/gytm
```
```
```
 
*Config file path (Linux):*
```
~/.config/ytm/config.json
```
# Usage
- Launch `gytm` and enjoy music
```
gytm
```
## 🛠 Troubleshooting
### Expired Cookies
If you encounter errors after using the app for a while, or if your **playlists and albums disappear**, it is likely that your YouTube Music session cookie has expired.

**To fix this:**
1. Follow the **Configuration** steps again to obtain a fresh cookie from your browser.
2. Update the `cookie` field in `~/.config/ytm/config.json`.
3. Restart `ytm`.
# ❤️ Credits & Inspiration
This project is inspired by: [ytermusic](https://github.com/ccgauche/ytermusic.git)
