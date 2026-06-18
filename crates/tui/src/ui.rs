use crate::app::{App, PopupState};
use crate::helper;
use crate::theme::Theme;
use api::protocol::ApiLoadingKind;
use data::AppPage::Library;
use data::{AppPage, CreatePlaylistFocus, FocusArea, PlayListPrivacy, PlayMode, PlayerStatus};
use ratatui::layout::Flex;
use ratatui::{
    self, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs},
};

pub fn render(app: &mut App, frame: &mut Frame, theme: &Theme, start_time: std::time::Instant) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            if app.page == Library {
                Constraint::Length(0)
            } else {
                Constraint::Length(3)
            },
            Constraint::Fill(1),
            Constraint::Percentage(30),
            Constraint::Length(4),
        ])
        .split(frame.area());
    let hor_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_layout[2]);

    let top_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
            Constraint::Length(2),
        ])
        .split(main_layout[0]);
    render_tabs(frame, top_layout[0], theme, app.page as usize);
    render_help_line(
        frame,
        top_layout[1],
        theme,
        vec![("Quit", "Q"), ("Minimize", "q"), ("Next tab", "Tab")],
    );
    render_queue(frame, app, main_layout[3], theme, start_time);
    render_player(frame, app, main_layout[4], theme);
    render_songs(frame, app, hor_layout[1], theme, start_time);

    match app.page {
        AppPage::Library => {
            // HORIZONTAL LAYOUT (ALBUMS | PLAYLISTS)
            let hor_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(main_layout[2]);
            let list_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(hor_layout[0]);

            render_list(
                frame,
                app,
                list_layout[0],
                FocusArea::Albums,
                theme,
                start_time,
            );
            render_list(
                frame,
                app,
                list_layout[1],
                FocusArea::Playlists,
                theme,
                start_time,
            );
        }
        AppPage::Search => {
            let result_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(hor_layout[0]);
            render_search_input(frame, app, main_layout[1], theme, start_time);
            render_search_albums(frame, app, result_layout[0], theme);
            render_search_songs(frame, app, result_layout[1], theme);
        }
    }
    match &app.popup_state {
        PopupState::None => {}
        PopupState::SaveSong { .. } => {
            render_save_song_to_playlist_popup(frame, app, frame.area(), theme, start_time);
        }
        PopupState::CreatePlaylist { .. } => {
            render_create_playlist_popup(frame, app, frame.area(), theme, start_time);
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
fn render_list(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    area_type: FocusArea,
    theme: &Theme,
    start_time: std::time::Instant,
) {
    let title = if area_type == FocusArea::Albums {
        "[1]- Albums"
    } else {
        "[2]-󰲸 Playlists"
    };
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
            Span::styled("Enter ", theme.key_style()),
            Span::styled("| Unsave: ", theme.text_style()),
            Span::styled("x ", theme.key_style()),
            Span::styled("]", theme.text_style()),
        ]);
        block = block.title_bottom(bottom_nav.alignment(ratatui::layout::Alignment::Center));
    } else {
        let bottom_nav = Line::from(vec![
            Span::styled("[ View content: ", theme.text_style()),
            Span::styled("l ", theme.key_style()),
            Span::styled("| Create playlist: ", theme.text_style()),
            Span::styled("a ", theme.key_style()),
            Span::styled("]", theme.text_style()),
        ]);
        block = block.title_bottom(bottom_nav.alignment(ratatui::layout::Alignment::Center));
    }

    if app.api_loading_kind == Some(ApiLoadingKind::FetchLibraryData) {
        let inner_area = block.inner(area);
        frame.render_widget(block, area);
        render_spinner(frame, inner_area, start_time);
        return;
    }
    let result = match area_type {
        FocusArea::Albums => Some((&app.albums, &mut app.albums_liststate)),
        FocusArea::Playlists => Some((&app.playlists, &mut app.playlists_liststate)),
        _ => None,
    };
    if let Some((data, list)) = result {
        let items: Vec<ListItem> = data
            .iter()
            .map(|item| {
                let is_playing = app
                    .playing_playlist_id
                    .as_ref()
                    .map_or_else(|| false, |playing| playing.as_str() == item.playlist_id);
                let content = if is_playing {
                    format!(" {} - {}", item.title, item.artist)
                } else {
                    format!("  {} - {}", item.title, item.artist)
                };
                if is_playing {
                    ListItem::new(content).style(Style::default().fg(theme.secondary))
                } else {
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
            .block(block)
            .highlight_style(highlight_style);

        frame.render_stateful_widget(list_widget, area, list);
    }
}

fn render_songs(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    start_time: std::time::Instant,
) {
    let is_focused = FocusArea::Songs == app.focus_area
        && !app.is_popup_active()
        && ((!app.is_insert && app.page == AppPage::Search) || app.page == AppPage::Library);
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };
    let items: Vec<ListItem> = app
        .songs
        .iter()
        .enumerate()
        .map(|(i, song)| {
            let content = format!(" {:>3}. {}", i + 1, song.title);
            ListItem::new(content)
        })
        .collect();
    let keymap = Line::from(vec![
        Span::styled("[ Save/Unsave song: ", theme.text_style()),
        Span::styled("x/X ", theme.key_style()),
        Span::styled("| Add to Queue: ", theme.text_style()),
        Span::styled("a ", theme.key_style()),
        Span::styled("]", theme.text_style()),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!("[4]-󰠶 Content ({})", app.songs.len()))
        .title_bottom(keymap.centered())
        .border_style(border_style);
    if app.api_loading_kind == Some(ApiLoadingKind::GetSongsToView) {
        let inner_area = block.inner(area);
        render_spinner(frame, inner_area, start_time);
        frame.render_widget(block, area);
        return;
    }
    let inner_area = block.inner(area);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(10),
        ])
        .split(inner_area);
    let list_title = if let Some(viewing_list) = &app.viewing_list {
        Line::from(format!(" {} - {}", viewing_list.title, viewing_list.artist))
    } else {
        Line::default()
    };
    let line = Block::default()
        .borders(Borders::TOP)
        .border_style(border_style);
    let highlight_style = if is_focused {
        theme.selected_item()
    } else {
        Style::default()
    };
    let list_widget = List::new(items).highlight_style(highlight_style);

    frame.render_widget(block, area);
    frame.render_widget(list_title, layout[0]);
    if app.viewing_list.is_some() {
        frame.render_widget(line, layout[1]);
    }
    frame.render_stateful_widget(list_widget, layout[2], &mut app.songs_liststate);
}

// RENDER QUEUE
fn render_queue(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    start_time: std::time::Instant,
) {
    let is_focused = FocusArea::Queue == app.focus_area
        && !app.is_popup_active()
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
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!("[3]- Queue ({})", app.queue.len()))
        .title_bottom(key_map.centered())
        .border_style(border_style);

    if app.api_loading_kind == Some(ApiLoadingKind::GetSongsToPlay) {
        let inner_area = block.inner(area);
        render_spinner(frame, inner_area, start_time);
        frame.render_widget(block, area);
        return;
    }
    if app.queue.is_empty() {
        frame.render_widget(block, area);
        return;
    }
    let items: Vec<ListItem> = app
        .queue
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
        .block(block)
        .highlight_style(highlight_style);

    frame.render_stateful_widget(list_widget, area, &mut app.queue_liststate);
}

fn render_player(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let song_info = match app.status {
        PlayerStatus::Idle => vec![Line::from("   No song is playing ".to_string())],
        _ => {
            let icon = if app.status == PlayerStatus::Playing {
                ""
            } else {
                ""
            };
            if let (Some(song), Some(time_pos)) = (&app.playing_song, app.time_pos) {
                let time_pos_text = helper::format_time(time_pos);
                vec![
                    Line::from(format!(" {}  {} ", icon, song.title)),
                    Line::from(format!("    {} / {}", time_pos_text, song.duration)),
                ]
            } else {
                vec![Line::from(String::new())]
            }
        }
    };
    let mode_text = match app.play_mode {
        PlayMode::DefaultMode => "Play mode:   Default ",
        PlayMode::ShuffleMode => "Play mode:   Shuffle ",
    };
    let right_content = vec![
        Line::from(mode_text),
        Line::from(format!("  {}% ", app.volume)),
    ];
    let key_map = Line::from(vec![
        Span::styled("[ ⏸ / : ", theme.text_style()),
        Span::styled("Space ", theme.key_style()),
        Span::styled("| Play mode: ", theme.text_style()),
        Span::styled("m ", theme.key_style()),
        Span::styled("|  / : ", theme.text_style()),
        Span::styled("b/n ", theme.key_style()),
        Span::styled("|  : ", theme.text_style()),
        Span::styled("+/- ", theme.key_style()),
        Span::styled("|  /  : ", theme.text_style()),
        Span::styled(" /  ", theme.key_style()),
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

fn render_search_input(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    start_time: std::time::Instant,
) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(3), Constraint::Percentage(100)])
        .split(area);
    let bottom_nav = Line::from(vec![
        Span::styled("[ Exit insert: ", theme.text_style()),
        Span::styled("Esc ", theme.key_style()),
        Span::styled("| Search: ", theme.text_style()),
        Span::styled("Enter", theme.key_style()),
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
    frame.render_widget(input, layout[1]);
    if app.api_loading_kind == Some(ApiLoadingKind::Search) {
        render_spinner(frame, layout[0], start_time);
    }
}

fn render_search_albums(frame: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = app
        .search_albums
        .iter()
        .map(|item| {
            let is_saved = match item.is_saved {
                true => "󰃂",
                false => " ",
            };
            let content = format!(" {} {} - {}", is_saved, item.title, item.artist);
            ListItem::new(content)
        })
        .collect();

    let is_focused =
        FocusArea::SearchAlbums == app.focus_area && !app.is_insert && !app.is_popup_active();
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };

    let bottom_nav = Line::from(vec![
        Span::styled("[ Save/Unsave: ", theme.text_style()),
        Span::styled("x ", theme.key_style()),
        Span::styled("| View content: ", theme.text_style()),
        Span::styled("l ", theme.key_style()),
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
            let content = format!("   {}", item.title);
            ListItem::new(content)
        })
        .collect();

    let keymap = Line::from(vec![
        Span::styled("[ Add to Queue: ", theme.text_style()),
        Span::styled("a ", theme.key_style()),
        Span::styled("| Save to Playlist: ", theme.text_style()),
        Span::styled("x ", theme.key_style()),
        Span::styled(" ]", theme.text_style()),
    ]);

    let is_focused =
        FocusArea::SearchSongs == app.focus_area && !app.is_insert && !app.is_popup_active();
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("[2]-󰎇 Songs")
        .title_bottom(keymap.centered())
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
fn render_save_song_to_playlist_popup(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    start_time: std::time::Instant,
) {
    let PopupState::SaveSong { selected_save_song } = &app.popup_state else {
        return;
    };

    let items: Vec<ListItem> = app
        .cus_playlists
        .iter()
        .filter_map(|&idx| app.playlists.get(idx))
        .map(|p| {
            if p.playlist_id == "LM" {
                ListItem::new(format!("  {}", p.title))
            } else {
                ListItem::new(format!(" 󰲸 {}", p.title))
            }
        })
        .collect();

    let keymap = Line::from(vec![
        Span::styled("[ Save: ", theme.text_style()),
        Span::styled("Enter ", theme.key_style()),
        Span::styled("| Close: ", theme.text_style()),
        Span::styled("Esc ", theme.key_style()),
        Span::styled("]", theme.text_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title_bottom(keymap.centered());

    let center_area = area.centered(Constraint::Percentage(40), Constraint::Percentage(50));
    let inner_area = block.inner(center_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner_area);
    let title_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(3), Constraint::Percentage(100)])
        .split(layout[0]);
    let list_widget = List::new(items).highlight_style(theme.selected_item());

    let title = Line::from(vec![
        Span::raw(" Saving "),
        Span::styled(&selected_save_song.title, theme.key_style()),
        Span::raw(" to:"),
    ]);
    let line = Block::default().borders(Borders::TOP);

    frame.render_widget(Clear, center_area);
    frame.render_widget(block, center_area);
    frame.render_widget(title, title_layout[1]);
    if app.api_loading_kind == Some(ApiLoadingKind::SaveToPlaylist) {
        render_spinner(frame, title_layout[0], start_time);
    }
    frame.render_widget(line, layout[1]);
    frame.render_stateful_widget(list_widget, layout[2], &mut app.cus_playlists_liststate);
}

fn render_create_playlist_popup(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    theme: &Theme,
    start_time: std::time::Instant,
) {
    let PopupState::CreatePlaylist {
        title,
        description,
        privacy,
        focused_field,
    } = &app.popup_state
    else {
        return;
    };

    let keymap = Line::from(vec![
        Span::styled("[ Tab: ", theme.text_style()),
        Span::styled("Next ", theme.key_style()),
        Span::styled("| Enter: ", theme.text_style()),
        Span::styled("Create ", theme.key_style()),
        Span::styled("| Esc: ", theme.text_style()),
        Span::styled("Cancel ", theme.key_style()),
        Span::styled("]", theme.text_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Create Playlist ")
        .title_bottom(keymap.centered());

    let center_area = area.centered(Constraint::Percentage(40), Constraint::Length(15));
    let inner_area = block.inner(center_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(4),
        ])
        .split(inner_area);
    frame.render_widget(Clear, center_area);
    frame.render_widget(block, center_area);
    if app.api_loading_kind == Some(ApiLoadingKind::CreatePlaylist) {
        render_spinner(frame, layout[0], start_time);
    }
    render_input_field(
        frame,
        layout[1],
        theme,
        " Title:",
        title,
        matches!(focused_field, CreatePlaylistFocus::Title),
    );
    render_input_field(
        frame,
        layout[2],
        theme,
        " Desc:",
        description,
        matches!(focused_field, CreatePlaylistFocus::Description),
    );
    render_privacy_selector(
        frame,
        layout[3],
        theme,
        privacy,
        matches!(focused_field, CreatePlaylistFocus::Privacy),
    );
}

fn render_input_field(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    label: &str,
    text: &str,
    is_focused: bool,
) {
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };

    let display = if is_focused {
        format!("{}_", text)
    } else {
        text.to_string()
    };

    let input = Paragraph::new(display)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(label)
                .border_style(border_style),
        )
        .style(Style::default().fg(theme.base));

    frame.render_widget(input, area);
}

fn render_privacy_selector(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    privacy: &PlayListPrivacy,
    is_focused: bool,
) {
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };

    let items = ["Private", "Public", "Unlisted"];
    let choices = [
        PlayListPrivacy::Private,
        PlayListPrivacy::Public,
        PlayListPrivacy::Unlisted,
    ];

    let spans: Vec<Span> = items
        .iter()
        .zip(choices.iter())
        .map(|(label, value)| {
            let selected = privacy == value;
            let prefix = if selected { " ● " } else { " ○ " };
            let text = format!("{}{}", prefix, label);
            if selected {
                Span::styled(text, theme.key_style())
            } else {
                Span::styled(text, theme.text_style())
            }
        })
        .collect();
    let keymap = Line::from(vec![
        Span::styled("[ Select previous: ", theme.text_style()),
        Span::styled("h/ ", theme.key_style()),
        Span::styled("| Select next: ", theme.text_style()),
        Span::styled("l/ ", theme.key_style()),
        Span::styled("]", theme.text_style()),
    ]);
    let p = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Privacy:")
            .title_bottom(keymap.centered())
            .border_style(border_style),
    );

    frame.render_widget(p, area);
}
fn render_spinner(f: &mut Frame, area: Rect, start_time: std::time::Instant) {
    let spinners = ["", "", "", "", "", ""];
    let elapsed = start_time.elapsed().as_millis();
    let index = ((elapsed / 80) as usize) % spinners.len();
    let [centered_area] = Layout::vertical([Constraint::Length(1)])
        .flex(Flex::Center)
        .areas(area);

    let spinner_widget = Paragraph::new(spinners[index])
        .style(Style::default().bold())
        .alignment(Alignment::Center);

    f.render_widget(spinner_widget, centered_area);
}
