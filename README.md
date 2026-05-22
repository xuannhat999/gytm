# gytm: TUI based Youtube Music player
Stream Youtube Music from your terminal !
# Demo 
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/42442ae5-a646-4c6e-93e0-39b9b7c0a40a" />

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
 
# Config file path (Linux):
```
~/.config/ytm/config.json
```
# Usage
- Launch `gytm` and enjoy music
```
gytm
```
# ❤️ Credits & Inspiration
This project is inspired by: [ytermusic](https://github.com/ccgauche/ytermusic.git)
