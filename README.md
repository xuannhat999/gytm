# ytm: TUI based Youtube Music player
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
git clone git@github.com:xuannhat999/ytm.git
```
2. Install the binary
```
cd  ytm
cargo install --path ./tui
```
3. Launch `ytm` once to generate the default configuration.
```
ytm
```
The app will exit automatically after creating the config file.

# Configuration (Required)
Fill your cookie in the config file
- Access [YouTube Music](https://music.youtube.com) in your browser ( make sure you logged in)
- Open Developer's Tools Tab (F12)
- Go to tab Network
- Choose filter `Fetch/XHR` under Filter bar
- Refresh page (F5)
- Look for `browse...`
- Choose it, choose Header tab
- Roll down to Request Headers section, there will bee a Cookie field, copy it
- Open the configuration file and paste to the `cookie` field  
*File path (Linux):*
```
~/.config/ytm/config.json
```
# Usage
- After fill in config file, launch `ytm` and enjoy music
```
ytm
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
