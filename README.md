# gytm: TUI based Youtube Music player
Stream Youtube Music from your terminal !
# Demo 
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/27178df9-703b-40ed-be15-8beebfb41b49" />
<br><br>

# Features
- Stream songs on Youtube Music as saved album/playlist
- Personalized Content: Seamlessly fetch your private playlists/album using local cookie authentication.

#  Packages required
- **yt-dlp**: For fetching stream URLs.
- **mpv**   : The core media engine.
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
3. Launch `gytm` once to generate the default configuration.
```
gytm
```
The app will exit automatically after creating the config file.

# Configuration (Required)
Fill your cookie in the config file
- Access [YouTube Music](https://music.youtube.com) in your browser ( make sure you signed in)
- Open Developer's Tools Tab (F12)
- Go to tab Network
- Choose filter `Fetch/XHR` under Filter bar
- Refresh page (F5)
- Find `browse...` and copy your Cookie at Request Headers in Headers tab
- Open the configuration file and paste to the `cookie` field  
*File path (Linux):*
```
~/.config/ytm/config.json
```
# Usage
- After fill in config file, launch `ytm` and enjoy music
```
gytm
```
## Troubleshooting
### Expired Cookies
If you encounter errors after using the app for a while, or if your **playlists and albums disappear**, it is likely that your YouTube Music session cookie has expired.

**To fix this:**
1. Follow the **Configuration** steps again to obtain a fresh cookie from your browser.
2. Update the `cookie` field in `~/.config/ytm/config.json`.
3. Restart `ytm`.
# ❤️ Credits & Inspiration
This project is inspired by: [ytermusic](https://github.com/ccgauche/ytermusic.git)
