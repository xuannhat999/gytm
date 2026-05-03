use crate::app::{App, FocusArea};
use player::PlayerState;
use ratatui::{
    self, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(app: &mut App, frame: &mut Frame) {
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
    render_list(frame, app, playlist_layout[0], FocusArea::Albums);
    render_list(frame, app, playlist_layout[1], FocusArea::Playlists);
    render_songs(frame, app, main_layout[1]);
    render_player(frame, app, main_layout[2]);
}

// RENDER ALBUM & PLAYLISTS
fn render_list(frame: &mut Frame, app: &mut App, area: Rect, area_type: FocusArea) {
    let result = match area_type {
        FocusArea::Albums => Some((&app.albums, &mut app.album_list_state, "[1]-Albums")),
        FocusArea::Playlists => Some((
            &app.playlists,
            &mut app.playlist_list_state,
            "[2]-Playlists",
        )),
        _ => None,
    };
    if let Some((data, state, title)) = result {
        let items: Vec<ListItem> = data
            .iter()
            .map(|list| ListItem::new(format!(" {} | {} ", list.title, list.artist)))
            .collect();
        let is_focused = app.focus_area == area_type;
        let border_style = if is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        // 4. Khởi tạo Widget List
        let list_widget = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(69, 71, 90))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list_widget, area, state);
    }
}

// RENDER SONGS FROM ALBUM/PLAYLIST
fn render_songs(frame: &mut Frame, app: &mut App, area: Rect) {
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
        .map(|song| ListItem::new(format!("{} ", song.title)))
        .collect();

    let is_focused = app.focus_area == FocusArea::SongList;

    let border_style = if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let highlight_style = if is_focused {
        Style::default()
            .bg(Color::Rgb(69, 71, 90)) // Màu sáng (Surface1)
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
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

// RENDER PLAYER
fn render_player(frame: &mut Frame, app: &mut App, area: Rect) {
    let content = if app.is_loading {
        "Fetching...".to_string()
    } else {
        if let Some(idx) = app.player.current_song_idx {
            if let Some(song) = app.songs.get(idx) {
                // Biểu tượng trạng thái
                let status_icon = if app.player.state == PlayerState::Playing {
                    "▶"
                } else {
                    "⏸"
                };
                format!(" {}  {} ", status_icon, song.title)
            } else {
                "  Unknown Track ".to_string()
            }
        } else {
            "  No song playing ".to_string()
        }
    };

    let key_map = Line::from(vec![
        Span::from(" <q> Quit"),
        Span::raw(" | "),
        Span::from("<Enter> Play"),
        Span::raw(" | "),
        Span::from("<Space> Pause/Resume"),
        Span::raw(" | "),
        Span::from("<n> Next_song"),
        Span::raw(" | "),
        Span::from("<p> Prev_song "),
    ]);
    let player_widget = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Player ")
                .title_bottom(key_map)
                .title_alignment(Alignment::Center)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(Color::White)),
        )
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left);

    // 4. Render lên màn hình
    frame.render_widget(player_widget, area);
}
