use crate::app::App;
use data::FocusArea;
use data::{PlayMode, PlayerStatus};
use player::Player;
use ratatui::{
    self, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};
struct Theme {
    primary: Color,
    secondary: Color,
    active: Color,
    inactive: Color,
    base: Color,
}
impl Default for Theme {
    fn default() -> Self {
        Theme {
            primary: Color::LightGreen,
            secondary: Color::LightYellow,
            active: Color::LightCyan,
            inactive: Color::DarkGray,
            base: Color::White,
        }
    }
}

pub fn render(app: &mut App, frame: &mut Frame, player: &Player) {
    let theme = Theme::default();
    // MAIN VERTICAL LAYOUT
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(50),
            Constraint::Percentage(10),
        ])
        .split(frame.area());

    // HORIZONTAL LAYOUT (ALBUMS | PLAYLISTS)
    let playlist_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_layout[0]);
    render_list(frame, app, playlist_layout[0], FocusArea::Albums, &theme);
    render_list(frame, app, playlist_layout[1], FocusArea::Playlists, &theme);
    render_songs(frame, app, main_layout[1], &theme);
    render_player(frame, app, main_layout[2], player, &theme);
}

// RENDER ALBUM & PLAYLISTS
fn render_list(frame: &mut Frame, app: &mut App, area: Rect, area_type: FocusArea, theme: &Theme) {
    let result = match area_type {
        FocusArea::Albums => Some((&app.albums, &mut app.album_list_state, "[1]- Albums")),
        FocusArea::Playlists => Some((
            &app.playlists,
            &mut app.playlist_list_state,
            "[2]-󰲸 Playlists",
        )),
        _ => None,
    };

    if let Some((data, list, title)) = result {
        let items: Vec<ListItem> = data
            .iter()
            .map(|item| {
                let is_playing = app
                    .playing_playlist
                    .as_ref()
                    .map_or(false, |playing| playing.as_str() == item.browse_id);
                let content = format!(" {} - {}", item.title, item.artist);
                if is_playing {
                    ListItem::new(content).style(Style::default().fg(theme.secondary))
                } else {
                    ListItem::new(content)
                }
            })
            .collect();

        let is_focused = app.focus_area == area_type;
        let border_style = if is_focused {
            Style::default()
                .fg(theme.active)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.inactive)
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(border_style);
        let key_style = Style::default()
            .fg(theme.secondary)
            .add_modifier(Modifier::BOLD);
        let text_style = Style::default().fg(theme.base);
        if area_type == FocusArea::Albums {
            let bottom_nav = Line::from(vec![
                Span::styled("[ Up: ", text_style),
                Span::styled("/k ", key_style),
                Span::styled("| Down: ", text_style),
                Span::styled("/j ", key_style),
                Span::styled("| Play/Select: ", text_style),
                Span::styled("<Enter>/l", key_style),
                Span::styled(" ]", text_style),
            ]);

            block = block.title_bottom(bottom_nav.alignment(ratatui::layout::Alignment::Right));
        }

        let list_widget = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(Color::Rgb(69, 71, 90))
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_stateful_widget(list_widget, area, list);
    }
}

// RENDER SONGS FROM ALBUM/PLAYLIST
fn render_songs(frame: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    if app.songs.is_empty() {
        let empty_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Tracks")
            .border_style(Style::default().fg(theme.inactive));

        let message = Paragraph::new("Select an Album or Playlist to view songs")
            .block(empty_block)
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(message, area);
        return;
    }
    let items: Vec<ListItem> = app
        .songs
        .iter()
        .enumerate()
        .map(|(i, song)| {
            let content = format!("{:>3}. {}", i + 1, song.title);
            if app
                .playing_song
                .as_ref()
                .map_or(false, |playing| playing.video_id == song.video_id)
            {
                ListItem::new(content).style(Style::default().fg(theme.primary))
            } else {
                ListItem::new(content)
            }
        })
        .collect();
    let is_focused = FocusArea::SongList == app.focus_area;

    let border_style = if is_focused {
        Style::default()
            .fg(theme.active)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.inactive)
    };
    let highlight_style = if is_focused {
        Style::default()
            .bg(Color::Rgb(69, 71, 90))
            .fg(theme.primary)
    } else {
        Style::default()
    };
    let list_widget = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!("[3]- Tracks ({})", app.songs.len()))
                .border_style(border_style),
        )
        .highlight_style(highlight_style)
        .highlight_symbol("▶");

    frame.render_stateful_widget(list_widget, area, &mut app.songs_list_state);
}

fn render_player(frame: &mut Frame, app: &App, area: Rect, player: &Player, theme: &Theme) {
    let song_info = if player.state == PlayerStatus::Loading {
        "Loading...".to_string()
    } else {
        if let Some(song) = &app.playing_song {
            let status_icon = if player.state == PlayerStatus::Playing {
                "▶ Playing: "
            } else {
                "⏸ Paused: "
            };
            format!(" {}  {} ", status_icon, song.title)
        } else {
            "  No song playing ".to_string()
        }
    };
    let mode_text = match player.play_mode {
        PlayMode::DefaultMode => "Play mode:    Default",
        PlayMode::ShuffleMode => "Play mode:    Shuffle",
    };
    let right_content = vec![
        Line::from(mode_text),
        Line::from(format!("Volume:    {}%", player.volume)),
    ];
    let key_style = Style::default()
        .fg(theme.secondary)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(theme.base);

    let key_map = Line::from(vec![
        Span::styled("[ Quit: ", text_style),
        Span::styled("q ", key_style),
        Span::styled("| Pause/Resume: ", text_style),
        Span::styled("Space ", key_style),
        Span::styled("| Pause/Resume: ", text_style),
        Span::styled("m", key_style),
        Span::styled("| Prev/Next: ", text_style),
        Span::styled("p/n ", key_style),
        Span::styled("| Volume: ", text_style),
        Span::styled("+/- ", key_style),
        Span::styled(" ]", text_style),
    ]);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .title(" Player")
        .title_bottom(key_map)
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.base));

    let left_area = Paragraph::new(song_info)
        .style(Style::default().fg(theme.base).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left);
    let right_area = Paragraph::new(right_content)
        .style(Style::default().fg(theme.base).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Right);

    frame.render_widget(main_block, area);

    let inner_area = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    let inner_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(inner_area);

    frame.render_widget(left_area, inner_chunks[0]);
    frame.render_widget(right_area, inner_chunks[1]);
}
