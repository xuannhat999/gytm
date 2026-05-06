use crate::app::{App, FocusArea};
use player::{PlayMode, Player, PlayerState};
use ratatui::{
    self, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};
struct CustomStyle {
    key_style: Style,
    text_style: Style,
    border_active: Style,
    border_inactive: Style,
    highlight_item: Style,
    playing_style: Style,
}
impl CustomStyle {
    fn new(theme: Theme) -> Self {
        CustomStyle {
            key_style: Style::default()
                .fg(theme.secondary_color)
                .add_modifier(Modifier::BOLD),
            text_style: Style::default().fg(Color::White),
            border_active: Style::default()
                .fg(theme.active_color)
                .add_modifier(Modifier::BOLD),
            border_inactive: Style::default().fg(theme.inactive_color),
            highlight_item: Style::default()
                .bg(Color::Rgb(69, 71, 90))
                .fg(theme.primary_color)
                .add_modifier(Modifier::BOLD),
            playing_style: Style::default(),
        }
    }
}
struct Theme {
    primary_color: Color,
    secondary_color: Color,
    active_color: Color,
    inactive_color: Color,
}
impl Default for Theme {
    fn default() -> Self {
        Theme {
            primary_color: Color::Magenta,
            secondary_color: Color::Yellow,
            active_color: Color::Cyan,
            inactive_color: Color::DarkGray,
        }
    }
}

pub fn render(app: &mut App, frame: &mut Frame, player: &Player) {
    let style = CustomStyle::new(Theme::default());
    // OUTER BORDER
    let main_block = Block::default()
        .borders(Borders::all())
        .border_type(ratatui::widgets::BorderType::Rounded);
    let inner_area = main_block.inner(frame.area());
    frame.render_widget(main_block, frame.area());

    // MAIN VERTICAL LAYOUT
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(50),
            Constraint::Percentage(10),
        ])
        .split(inner_area);

    // HORIZONTAL LAYOUT (ALBUMS | PLAYLISTS)
    let playlist_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_layout[0]);
    render_list(frame, app, playlist_layout[0], FocusArea::Albums, &style);
    render_list(frame, app, playlist_layout[1], FocusArea::Playlists, &style);
    render_songs(frame, app, main_layout[1], &style);
    render_player(frame, app, main_layout[2], player, &style);
}

// RENDER ALBUM & PLAYLISTS
fn render_list(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    area_type: FocusArea,
    style: &CustomStyle,
) {
    let result = match area_type {
        FocusArea::Albums => Some((&app.albums, &mut app.album_list_state, "[1]-Albums")),
        FocusArea::Playlists => Some((
            &app.playlists,
            &mut app.playlist_list_state,
            "[2]-Playlists",
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
                    ListItem::new(content).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ListItem::new(content)
                }
            })
            .collect();

        let is_focused = app.focus_area == area_type;
        let border_style = if is_focused {
            style.border_active
        } else {
            style.border_inactive
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style);

        if area_type == FocusArea::Albums {
            let bottom_nav = Line::from(vec![
                Span::styled("</k>", style.key_style),
                Span::styled(" Up", style.text_style),
                Span::styled(" | ", style.text_style),
                Span::styled("</j>", style.key_style),
                Span::styled(" Down", style.text_style),
                Span::styled(" | ", style.text_style),
                Span::styled("<Enter/l>", style.key_style),
                Span::styled(" Play/Toggle", style.text_style),
            ]);

            block = block.title_bottom(bottom_nav.alignment(ratatui::layout::Alignment::Right));
        }

        let list_widget = List::new(items)
            .block(block)
            .highlight_style(style.highlight_item);

        frame.render_stateful_widget(list_widget, area, list);
    }
}

// RENDER SONGS FROM ALBUM/PLAYLIST
fn render_songs(frame: &mut Frame, app: &mut App, area: Rect, style: &CustomStyle) {
    if app.songs.is_empty() {
        let empty_block = Block::default()
            .borders(Borders::ALL)
            .title("Songs")
            .border_style(Style::default().fg(Color::DarkGray));

        let message = Paragraph::new("Select an Album or Playlist to view songs")
            .block(empty_block)
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(message, area);
        return;
    }
    let items: Vec<ListItem> = app
        .songs
        .iter()
        .map(|song| {
            let is_playing = app
                .playing_song
                .as_ref()
                .map_or(false, |playing| playing.video_id == song.video_id);

            let content = format!(" {}", song.title);

            if is_playing {
                ListItem::new(content).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(content)
            }
        })
        .collect();
    let is_focused = FocusArea::SongList == app.focus_area;

    let border_style = if is_focused {
        style.border_active
    } else {
        style.border_inactive
    };
    let highlight_style = if is_focused {
        style.highlight_item
    } else {
        Style::default()
    };
    let list_widget = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("[3]-Tracks ({})", app.songs.len()))
                .border_style(border_style),
        )
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list_widget, area, &mut app.songs_list_state);
}

fn render_player(frame: &mut Frame, app: &App, area: Rect, player: &Player, style: &CustomStyle) {
    let song_info = if player.state == PlayerState::Loading {
        "Loading...".to_string()
    } else {
        if let Some(song) = &app.playing_song {
            let status_icon = if player.state == PlayerState::Playing {
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

    let key_map = Line::from(vec![
        Span::styled(" <q> ", style.key_style),
        Span::styled("Quit", style.text_style),
        Span::styled(" | ", style.text_style),
        Span::styled("<Space> ", style.key_style),
        Span::styled("Pause/Resume", style.text_style),
        Span::styled(" | ", style.text_style),
        Span::styled("<m> ", style.key_style),
        Span::styled("PlayMode", style.text_style),
        Span::styled(" | ", style.text_style),
        Span::styled("<p/n> ", style.key_style),
        Span::styled("Prev/Next", style.text_style),
    ]);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .title(" Player ")
        .title_bottom(key_map)
        .title_alignment(Alignment::Center)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White));

    let left_area = Paragraph::new(song_info)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left);
    let right_area = Paragraph::new(mode_text)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
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
