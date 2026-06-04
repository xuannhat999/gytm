use crate::app::App;
use crate::theme::Theme;
use data::{AppPage, FocusArea};
use data::{PlayMode, PlayerStatus};
use player::Player;
use ratatui::widgets::Tabs;
use ratatui::{
    self, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};

pub fn render(app: &mut App, frame: &mut Frame, player: &Player, theme: &Theme) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(70),
            Constraint::Percentage(30),
            Constraint::Length(4),
        ])
        .split(frame.area());
    let top_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_layout[0]);
    render_tabs(frame, top_layout[0], theme, app.page as usize);
    render_help_line(
        frame,
        top_layout[1],
        theme,
        vec![("Quit", "q"), ("Next tab", "Tab")],
    );
    render_queue(frame, app, main_layout[2], theme);
    render_player(frame, app, main_layout[3], player, theme);
    match app.page {
        AppPage::Library => {
            // HORIZONTAL LAYOUT (ALBUMS | PLAYLISTS)
            let playlist_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_layout[1]);
            render_list(frame, app, playlist_layout[0], FocusArea::Albums, theme);
            render_list(frame, app, playlist_layout[1], FocusArea::Playlists, theme);
        }
        AppPage::Search => {
            let search_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Percentage(100)])
                .split(main_layout[1]);
            let result_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(search_layout[1]);
            render_search_input(frame, app, search_layout[0], theme);
            render_search_albums(frame, app, result_layout[0], theme);
            render_search_songs(frame, app, result_layout[1], theme);
        }
    }
}

fn render_help_line(frame: &mut Frame, area: Rect, theme: &Theme, items: Vec<(&str, &str)>) {
    let mut spans = Vec::new();
    for (i, (desc, key)) in items.iter().enumerate() {
        spans.push(Span::styled(format!("{}: ", desc), theme.text_style()));
        spans.push(Span::styled(format!("[{}]", key), theme.key_style()));
        if i < items.len() - 1 {
            spans.push(Span::styled(" | ", theme.text_style()));
        }
    }
    let p = Paragraph::new(Line::from(spans)).alignment(Alignment::Right);
    frame.render_widget(p, area);
}
fn render_tabs(frame: &mut Frame, area: Rect, theme: &Theme, current_idx: usize) {
    let titles = vec![Line::from("  Library "), Line::from("  Search ")];
    let tabs = Tabs::new(titles)
        .style(theme.text_style())
        .highlight_style(
            theme
                .key_style()
                .bg(theme
                    .text_style()
                    .fg
                    .unwrap_or(ratatui::style::Color::DarkGray))
                .fg(ratatui::style::Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .select(current_idx)
        .divider("|");
    frame.render_widget(tabs, area);
}

// RENDER ALBUM & PLAYLISTS
fn render_list(frame: &mut Frame, app: &mut App, area: Rect, area_type: FocusArea, theme: &Theme) {
    let result = match area_type {
        FocusArea::Albums => Some((&app.albums, &mut app.albums_liststate, "[1]- Albums")),
        FocusArea::Playlists => Some((
            &app.playlists,
            &mut app.playlists_liststate,
            "[2]-󰲸 Playlists",
        )),
        _ => None,
    };

    if let Some((data, list, title)) = result {
        let items: Vec<ListItem> = data
            .iter()
            .map(|item| {
                let is_playing = app
                    .playing_playlist_id
                    .as_ref()
                    .map_or_else(|| false, |playing| playing.as_str() == item.browse_id);
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
            theme.active_border_style()
        } else {
            theme.inactive_border_style()
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(border_style);
        if area_type == FocusArea::Albums {
            let bottom_nav = Line::from(vec![
                Span::styled("[ Up: ", theme.text_style()),
                Span::styled("/k ", theme.key_style()),
                Span::styled("| Down: ", theme.text_style()),
                Span::styled("/j ", theme.key_style()),
                Span::styled("| Play: ", theme.text_style()),
                Span::styled("<Enter>", theme.key_style()),
                Span::styled(" ]", theme.text_style()),
            ]);

            block = block.title_bottom(bottom_nav.alignment(ratatui::layout::Alignment::Center));
        }
        let highlight_style = if is_focused {
            theme.selected_item()
        } else {
            Style::default()
        };

        let list_widget = List::new(items)
            .block(block)
            .highlight_style(highlight_style);

        frame.render_stateful_widget(list_widget, area, list);
    }
}

// RENDER QUEUE
fn render_queue(frame: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let is_focused = FocusArea::Queue == app.focus_area
        && ((!app.is_insert && app.page == AppPage::Search) || app.page == AppPage::Library);
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };
    let key_map = Line::from(vec![
        Span::styled("[ Remove from queue: ", theme.text_style()),
        Span::styled("d ", theme.key_style()),
        Span::styled("| Clear Queue: ", theme.text_style()),
        Span::styled("c ", theme.key_style()),
        Span::styled("]", theme.text_style()),
    ]);

    if app.songs.is_empty() {
        let empty_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("[3]- Queue")
            .title_bottom(key_map.centered())
            .border_style(border_style);

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
            if Option::map_or(app.playing_song.as_ref(), false, |playing| {
                playing.video_id == song.video_id
            }) {
                let content = format!("{:>3}. {}", i + 1, song.title);
                ListItem::new(content).style(Style::default().fg(theme.primary))
            } else {
                let content = format!(" {:>3}. {}", i + 1, song.title);
                ListItem::new(content)
            }
        })
        .collect();
    let highlight_style = if is_focused {
        theme.selected_item()
    } else {
        Style::default()
    };

    let list_widget = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!("[3]- Queue ({})", app.songs.len()))
                .title_bottom(key_map.centered())
                .border_style(border_style),
        )
        .highlight_style(highlight_style);

    frame.render_stateful_widget(list_widget, area, &mut app.songs_liststate);
}

fn render_player(frame: &mut Frame, app: &App, area: Rect, player: &Player, theme: &Theme) {
    let song_info = match player.status {
        PlayerStatus::Idle => "   No song is playing ".to_string(),
        PlayerStatus::Loading => "   Loading...".to_string(),
        PlayerStatus::Playing => {
            if let Some(song) = &app.playing_song {
                format!("   {} ", song.title)
            } else {
                String::new()
            }
        }
        PlayerStatus::Paused => {
            if let Some(song) = &app.playing_song {
                format!(" ⏸  {} ", song.title)
            } else {
                String::new()
            }
        }
    };
    let mode_text = match player.play_mode {
        PlayMode::DefaultMode => "Play mode:   Default ",
        PlayMode::ShuffleMode => "Play mode:   Shuffle ",
    };
    let right_content = vec![
        Line::from(mode_text),
        Line::from(format!("  {}% ", player.volume)),
    ];
    let key_map = Line::from(vec![
        Span::styled("[ ⏸ / : ", theme.text_style()),
        Span::styled("<Space> ", theme.key_style()),
        Span::styled("| Play mode: ", theme.text_style()),
        Span::styled("m ", theme.key_style()),
        Span::styled("| 󰒮 / 󰒭: ", theme.text_style()),
        Span::styled("b/n ", theme.key_style()),
        Span::styled("|  : ", theme.text_style()),
        Span::styled("+/- ", theme.key_style()),
        Span::styled(" ]", theme.text_style()),
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

    let inner_area = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    let inner_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(inner_area);

    frame.render_widget(main_block, area);
    frame.render_widget(left_area, inner_chunks[0]);
    frame.render_widget(right_area, inner_chunks[1]);
}

fn render_search_input(frame: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let bottom_nav = Line::from(vec![
        Span::styled("[ Exit insert: ", theme.text_style()),
        Span::styled("Esc ", theme.key_style()),
        Span::styled("| Search: ", theme.text_style()),
        Span::styled("<Enter>", theme.key_style()),
        Span::styled(" ]", theme.text_style()),
    ]);
    let display_text = if app.is_insert {
        format!("{}_", app.search_query)
    } else {
        app.search_query.clone()
    };
    let input = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("[s]-󰍉 Search")
                .border_style(if app.is_insert {
                    theme.active_border_style()
                } else {
                    theme.inactive_border_style()
                })
                .title_bottom(bottom_nav.alignment(ratatui::layout::HorizontalAlignment::Center)),
        )
        .style(Style::default().fg(theme.base));
    frame.render_widget(input, area);
}

fn render_search_albums(frame: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = app
        .search_albums
        .iter()
        .map(|item| {
            let is_saved = match item.is_saved {
                true => "󰃂 ",
                false => "  ",
            };
            let content = format!("{} {} - {}", is_saved, item.title, item.artist);
            ListItem::new(content)
        })
        .collect();

    let is_focused = FocusArea::SearchAlbums == app.focus_area && !app.is_insert;
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };

    let bottom_nav = Line::from(vec![
        Span::styled("[ Save to Lib: ", theme.text_style()),
        Span::styled("a ", theme.key_style()),
        Span::styled("| Remove from Lib: ", theme.text_style()),
        Span::styled("d ", theme.key_style()),
        Span::styled(" ]", theme.text_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("[1]- Albums")
        .border_style(border_style)
        .title_bottom(bottom_nav.alignment(ratatui::layout::Alignment::Center));
    let list_widget = List::new(items)
        .block(block)
        .highlight_style(if is_focused {
            theme.selected_item()
        } else {
            Style::default()
        });

    frame.render_stateful_widget(list_widget, area, &mut app.search_albums_liststate);
}

fn render_search_songs(frame: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = app
        .search_songs
        .iter()
        .map(|item| {
            let content = item.title.to_string();
            ListItem::new(content)
        })
        .collect();

    let bottom_nav = Line::from(vec![
        Span::styled("[ Add to Queue: ", theme.text_style()),
        Span::styled("a ", theme.key_style()),
        Span::styled(" ]", theme.text_style()),
    ]);

    let is_focused = FocusArea::SearchSongs == app.focus_area && !app.is_insert;
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("[2]-󰎇 Songs")
        .title_bottom(bottom_nav.centered())
        .border_style(border_style);

    let list_widget = List::new(items)
        .block(block)
        .highlight_style(if is_focused {
            theme.selected_item()
        } else {
            Style::default()
        });

    frame.render_stateful_widget(list_widget, area, &mut app.search_songs_liststate);
}
