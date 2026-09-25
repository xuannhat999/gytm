use crate::app::App;
use crate::helper;
use api::protocol::ApiLoadingKind;
use config::Config;
use data::api_client::{ALL_BROWSERS, BrowserEngine};
use data::app::{
    AppPage, CreatePlaylistFocus, FocusArea, PlayListPrivacy, PlayMode, PlayerStatus, PopupState,
    SearchSongSource,
};
use data::theme::Theme;
use ratatui::layout::Flex;
use ratatui::style::Color;
use ratatui::widgets::{Row, Table};
use ratatui::{
    self, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

pub fn render(app: &mut App, frame: &mut Frame, config: &Config, start_time: std::time::Instant) {
    if config.background {
        render_background(frame, frame.area(), config.theme.bg);
    }
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            if app.page == AppPage::Library {
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
        .constraints([Constraint::Length(28), Constraint::Percentage(100)])
        .split(main_layout[0]);

    render_tabs(frame, top_layout[0], &config.theme, app.page as usize);
    render_help_line(
        frame,
        top_layout[1],
        &config.theme,
        vec![
            ("YTM client", "i"),
            ("Next tab", "Tab"),
            ("Minimize", "q"),
            ("Quit", "Q"),
        ],
    );
    render_queue(frame, app, main_layout[3], &config.theme, start_time);
    render_player(frame, app, main_layout[4], &config.theme);
    render_songs(frame, app, hor_layout[1], &config.theme, start_time);

    match app.page {
        AppPage::Library => {
            // HORIZONTAL LAYOUT (ALBUMS | PLAYLISTS)
            let list_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(hor_layout[0]);

            render_list(
                frame,
                app,
                list_layout[0],
                FocusArea::Albums,
                &config.theme,
                start_time,
            );
            render_list(
                frame,
                app,
                list_layout[1],
                FocusArea::Playlists,
                &config.theme,
                start_time,
            );
        }
        AppPage::Search => {
            let result_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(hor_layout[0]);
            render_search_bar(frame, app, main_layout[1], &config.theme, start_time);
            render_search_albums(frame, app, result_layout[0], &config.theme);
            render_search_songs(frame, app, result_layout[1], &config.theme);
        }
    }

    match &app.popup_state {
        PopupState::None => {}
        PopupState::SelectGeckoContainer { .. } => {
            render_select_gecko_container_popup(frame, app, frame.area(), config, start_time);
        }
        PopupState::SelectBrowserProfile { .. } => {
            render_select_profile_popup(frame, app, frame.area(), config, start_time)
        }
        PopupState::SaveSong { .. } => {
            render_save_song_to_playlist_popup(frame, app, frame.area(), config, start_time);
        }
        PopupState::CreatePlaylist { .. } => {
            render_create_playlist_popup(frame, app, frame.area(), config, start_time);
        }
        PopupState::SelectBrowser => {
            render_select_browser_popup(frame, app, frame.area(), config);
        }
        PopupState::SelectAccount { .. } => {
            render_select_account_popup(frame, app, frame.area(), config);
        }
        PopupState::ApiCLient => {
            render_api_client_popup(frame, frame.area(), config, app, start_time);
        }
    }
}

fn render_help_line(frame: &mut Frame, area: Rect, theme: &Theme, items: Vec<(&str, &str)>) {
    let mut spans = Vec::new();
    for (i, (desc, key)) in items.iter().enumerate() {
        spans.push(Span::styled(format!("{}: ", desc), theme.text_style()));
        spans.push(Span::styled(key.to_string(), theme.key_style()));
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
        .highlight_style(
            Style::default()
                .bg(theme.active)
                .fg(theme.bg)
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
    let is_focused = app.focus_area == area_type && !app.is_popup_active();
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

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if app.api_loading_kind == Some(ApiLoadingKind::FetchLibraryData) {
        render_spinner(frame, inner_area, theme, start_time);
        return;
    }
    if app.albums.is_empty() && app.playlists.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                "Currently in Guest mode, library features are unavailable",
                theme.text_style(),
            )),
            Line::from(vec![
                Span::styled("Press ", theme.text_style()),
                Span::styled("[i]", theme.key_style()),
                Span::styled(" to check YTM client configuration", theme.text_style()),
            ]),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(msg, inner_area);
        return;
    }
    let result = match area_type {
        FocusArea::Albums => Some((&app.albums, &mut app.albums_tablestate)),
        FocusArea::Playlists => Some((&app.playlists, &mut app.playlists_tablestate)),
        _ => None,
    };
    if let Some((data, list)) = result {
        let rows = data.iter().map(|item| {
            let playing = app.playing_playlist_id.as_deref().map_or("", |id| {
                if id == item.playlist_id.as_str() {
                    " "
                } else {
                    ""
                }
            });
            Row::new([playing, item.title.as_str(), item.artist.as_str()])
        });

        let highlight_style = if is_focused {
            theme.selected_item()
        } else {
            Style::default()
        };
        let colum_width = [
            Constraint::Length(2),
            Constraint::Percentage(80),
            Constraint::Percentage(20),
        ];
        let table = Table::new(rows, colum_width)
            .header(Row::new(["", "Title", "Artist"]))
            .row_highlight_style(highlight_style);
        frame.render_stateful_widget(table, inner_area, list);
    }
}

// VIEW SONGS
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
            let content = format!(" {:>3}. {} - {}", i + 1, song.title, song.artist);
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
        render_spinner(frame, inner_area, theme, start_time);
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
    let highlight_style = if is_focused {
        theme.selected_item()
    } else {
        Style::default()
    };
    let list_widget = List::new(items).highlight_style(highlight_style);

    frame.render_widget(block, area);
    frame.render_widget(list_title, layout[0]);
    if app.viewing_list.is_some() {
        render_v_divider(frame, layout[1], Borders::BOTTOM, border_style);
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
        render_spinner(frame, inner_area, theme, start_time);
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
            if app.playing_song_idx.is_some_and(|playing| playing == i) {
                let content = format!("{:>3}. {} - {}", i + 1, song.title, song.artist);
                ListItem::new(content).style(Style::default().fg(theme.primary))
            } else {
                let content = format!(" {:>3}. {} - {}", i + 1, song.title, song.artist);
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

// MPV PLAYER
fn render_player(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let song_info = match app.player_status {
        PlayerStatus::Idle => vec![Line::from("   No song is playing ".to_string())],
        _ => {
            let icon = if app.player_status == PlayerStatus::Playing {
                ""
            } else {
                ""
            };
            if let (Some(idx), Some(time_pos)) = (app.playing_song_idx, app.time_pos)
                && idx < app.queue.len()
            {
                let time_pos_text = helper::format_time(time_pos);
                vec![
                    Line::from(format!(
                        " {}  {} - {} ",
                        icon, app.queue[idx].title, app.queue[idx].artist
                    )),
                    Line::from(format!(
                        "    {} / {}",
                        time_pos_text, app.queue[idx].duration
                    )),
                ]
            } else {
                vec![Line::from(String::new())]
            }
        }
    };
    let mode_text = match app.player_state.play_mode {
        PlayMode::DefaultMode => "Play mode:   Default ",
        PlayMode::ShuffleMode => "Play mode:   Shuffle ",
    };
    let right_content = vec![
        Line::from(mode_text),
        Line::from(format!("  {}% ", app.player_state.volume)),
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

// SEARCH BAR
fn render_search_bar(
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
    let line = if app.is_insert {
        Line::from(vec![Span::raw(app.search_query.as_str()), Span::raw("_")])
    } else {
        Line::from(app.search_query.as_str())
    };

    let input = Paragraph::new(line)
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
        render_spinner(frame, layout[0], theme, start_time);
    }
}

// SEARCH ALBUM RESULTS
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
        .get_search_songs_from_source()
        .iter()
        .map(|item| {
            let content = format!("   {} - {}", item.title, item.artist);
            ListItem::new(content)
        })
        .collect();

    let keymap = Line::from(vec![
        Span::styled("[ Add to Queue: ", theme.text_style()),
        Span::styled("a ", theme.key_style()),
        Span::styled("| Save to Playlist: ", theme.text_style()),
        Span::styled("x ", theme.key_style()),
        Span::styled("| Switch type: ", theme.text_style()),
        Span::styled("h/l", theme.key_style()),
        Span::styled(" ]", theme.text_style()),
    ]);

    let is_focused =
        FocusArea::SearchSongs == app.focus_area && !app.is_insert && !app.is_popup_active();
    let border_style = if is_focused {
        theme.active_border_style()
    } else {
        theme.inactive_border_style()
    };
    let tab_hl = Style::default()
        .bg(if is_focused {
            theme.active
        } else {
            theme.inactive
        })
        .fg(theme.bg)
        .add_modifier(Modifier::BOLD);

    let (song_style, video_style) = match app.search_songs_source {
        SearchSongSource::Song => (tab_hl, border_style),
        SearchSongSource::Video => (border_style, tab_hl),
    };
    let title = Line::from(vec![
        Span::styled("[2]-", border_style),
        Span::styled(" 󰎇 Songs ", song_style),
        Span::styled("|", border_style),
        Span::styled("  Videos ", video_style),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_bottom(keymap.centered())
        .border_style(border_style);

    let list_widget = List::new(items)
        .block(block)
        .highlight_style(if is_focused {
            theme.selected_item()
        } else {
            Style::default()
        });

    frame.render_stateful_widget(
        list_widget,
        area,
        app.get_search_songs_liststate_from_source(),
    );
}

// SAVE SONG TO PLAYLIST
fn render_save_song_to_playlist_popup(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    config: &Config,
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
        Span::styled("[ Save: ", config.theme.text_style()),
        Span::styled("Enter ", config.theme.key_style()),
        Span::styled("| Close: ", config.theme.text_style()),
        Span::styled("Esc ", config.theme.key_style()),
        Span::styled("]", config.theme.text_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(config.theme.active_border_style())
        .title_bottom(keymap.centered());

    let center_area = area.centered(Constraint::Percentage(50), Constraint::Percentage(50));
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
    let list_widget = List::new(items).highlight_style(config.theme.selected_item());

    let title = Line::from(vec![
        Span::raw(" Saving "),
        Span::styled(&selected_save_song.title, config.theme.key_style()),
        Span::raw(" to:"),
    ]);
    let line = Block::default()
        .borders(Borders::TOP)
        .border_style(config.theme.active_border_style());

    frame.render_widget(Clear, center_area);
    if config.background {
        render_background(frame, center_area, config.theme.bg_popup);
    }
    frame.render_widget(block, center_area);
    frame.render_widget(title, title_layout[1]);
    if app.api_loading_kind == Some(ApiLoadingKind::SaveToPlaylist) {
        render_spinner(frame, title_layout[0], &config.theme, start_time);
    }
    frame.render_widget(line, layout[1]);
    frame.render_stateful_widget(list_widget, layout[2], &mut app.cus_playlists_liststate);
}

// API CLIENT SELECT
fn select_keymap(theme: &Theme) -> Line<'_> {
    Line::from(vec![
        Span::styled("[ Select: ", theme.text_style()),
        Span::styled("Enter/l/ ", theme.key_style()),
        Span::styled("| Back: ", theme.text_style()),
        Span::styled("h/ ", theme.key_style()),
        Span::styled("| Cancel: ", theme.text_style()),
        Span::styled("Esc ", theme.key_style()),
        Span::styled(" ]", theme.text_style()),
    ])
}

fn render_side_info(frame: &mut Frame, area: Rect, lines: Vec<String>) {
    frame.render_widget(
        Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<Line>>())
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_v_divider(frame: &mut Frame, area: Rect, border: Borders, border_style: Style) {
    frame.render_widget(
        Block::default().borders(border).border_style(border_style),
        area,
    );
}
fn render_api_client_popup(
    frame: &mut Frame,
    area: Rect,
    config: &Config,
    app: &App,
    start_time: std::time::Instant,
) {
    let keymap = vec![
        Span::styled("[ Reload: ", config.theme.text_style()),
        Span::styled("r ", config.theme.key_style()),
        Span::styled("| Guest: ", config.theme.text_style()),
        Span::styled("g ", config.theme.key_style()),
        Span::styled("| Close: ", config.theme.text_style()),
        Span::styled("Esc ", config.theme.key_style()),
        Span::styled("]", config.theme.text_style()),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(config.theme.active_border_style())
        .title(" YTM client 󱛜 ")
        .title_bottom(Line::from(keymap).centered());

    let center_area = area.centered(Constraint::Percentage(40), Constraint::Length(20));
    let inner_area = block.inner(center_area);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Max(4), Constraint::Min(0)])
        .split(inner_area);

    frame.render_widget(Clear, center_area);
    if config.background {
        render_background(frame, center_area, config.theme.bg_popup);
    }
    frame.render_widget(block, center_area);
    if let Ok((browser, profile, gecko_container, account)) =
        app.client_state.get_validated_fields()
    {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("[b] ", config.theme.key_style()),
                Span::styled(format!("Browser: {:?}", browser), config.theme.text_style()),
            ]),
            Line::from(vec![
                Span::styled("[p] ", config.theme.key_style()),
                Span::styled(
                    format!("Profile: {}", profile.name),
                    config.theme.text_style(),
                ),
            ]),
        ];
        if browser.engine() == BrowserEngine::Gecko {
            lines.push(Line::from(vec![
                Span::styled("[c] ", config.theme.key_style()),
                Span::styled(
                    format!(
                        "Container: {}",
                        gecko_container.map_or("None", |c| c.name.as_str())
                    ),
                    config.theme.text_style(),
                ),
            ]));
        }
        lines.push(Line::from(vec![
            Span::styled("[a] ", config.theme.key_style()),
            Span::styled(
                format!("Account: {}", account.map_or("None", |a| a.email.as_str())),
                config.theme.text_style(),
            ),
        ]));
        let p = Paragraph::new(lines).alignment(Alignment::Left);
        frame.render_widget(p, layout[0]);
        if matches!(
            app.api_loading_kind,
            Some(ApiLoadingKind::ReloadClient) | Some(ApiLoadingKind::FetchAccountsList)
        ) {
            render_spinner(frame, layout[1], &config.theme, start_time);
        }
    } else {
        if app.api_loading_kind == Some(ApiLoadingKind::ReloadClient) {
            render_spinner(frame, inner_area, &config.theme, start_time);
        } else {
            let lines = vec![
                Line::from(Span::styled(
                    "Currently running in Guest mode",
                    config.theme.text_style(),
                )),
                Line::from(vec![
                    Span::styled("[b] ", config.theme.key_style()),
                    Span::styled("Setup YTM client", config.theme.text_style()),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines).centered(), inner_area);
        }
    }
}
// GECKO CONTAINER
fn render_select_gecko_container_popup(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    config: &Config,
    start_time: std::time::Instant,
) {
    let PopupState::SelectGeckoContainer {
        containers,
        containers_liststate,
        browser,
        profile,
    } = &mut app.popup_state
    else {
        return;
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(config.theme.active_border_style())
        .title(" Container  ")
        .title_bottom(select_keymap(&config.theme).centered());

    let center_area = area.centered(Constraint::Percentage(50), Constraint::Length(20));
    let inner_area = block.inner(center_area);
    let hor_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Length(1),
            Constraint::Percentage(30),
        ])
        .split(inner_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(hor_layout[0]);

    frame.render_widget(Clear, center_area);
    if config.background {
        render_background(frame, center_area, config.theme.bg_popup);
    }
    frame.render_widget(block, center_area);
    let items: Vec<ListItem> = containers
        .iter()
        .map(|c| ListItem::new(format!(" {}", c.name)))
        .collect();

    let list_widget = List::new(items).highlight_style(config.theme.selected_item());
    if app.api_loading_kind == Some(ApiLoadingKind::FetchAccountsList) {
        render_spinner(frame, layout[0], &config.theme, start_time);
    }
    render_side_info(
        frame,
        hor_layout[2],
        vec![
            format!("  : {:?}", *browser),
            format!("  : {}", profile.name),
        ],
    );
    render_v_divider(
        frame,
        hor_layout[1],
        Borders::LEFT,
        config.theme.active_border_style(),
    );
    frame.render_stateful_widget(list_widget, layout[1], containers_liststate);
}

// PROFILE
fn render_select_profile_popup(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    config: &Config,
    start_time: std::time::Instant,
) {
    let PopupState::SelectBrowserProfile {
        profiles,
        profiles_liststate,
        browser,
    } = &mut app.popup_state
    else {
        return;
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(config.theme.active_border_style())
        .title(" Profile  ")
        .title_bottom(select_keymap(&config.theme).centered());

    let center_area = area.centered(Constraint::Percentage(50), Constraint::Length(20));
    let inner_area = block.inner(center_area);
    let hor_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Length(1),
            Constraint::Percentage(30),
        ])
        .split(inner_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(hor_layout[0]);

    frame.render_widget(Clear, center_area);
    if config.background {
        render_background(frame, center_area, config.theme.bg_popup);
    }
    frame.render_widget(block, center_area);

    if app.api_loading_kind == Some(ApiLoadingKind::FetchAccountsList) {
        render_spinner(frame, layout[0], &config.theme, start_time);
    }
    render_side_info(frame, hor_layout[2], vec![format!("  : {:?}", *browser)]);
    render_v_divider(
        frame,
        hor_layout[1],
        Borders::LEFT,
        config.theme.active_border_style(),
    );

    if profiles.is_empty() {
        let msg = Paragraph::new("No profiles found")
            .style(config.theme.text_style())
            .alignment(Alignment::Center);
        frame.render_widget(msg, layout[1]);
        return;
    }
    let items: Vec<ListItem> = profiles
        .iter()
        .map(|p| ListItem::new(format!(" {}", p.name)))
        .collect();
    let list_widget = List::new(items).highlight_style(config.theme.selected_item());

    frame.render_stateful_widget(list_widget, layout[1], profiles_liststate);
}

// BROWSER
fn render_select_browser_popup(frame: &mut Frame, app: &mut App, area: Rect, config: &Config) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(config.theme.active_border_style())
        .title_bottom(select_keymap(&config.theme).centered())
        .title(" Browser  ");
    let center_area = area.centered(Constraint::Percentage(50), Constraint::Length(20));
    let inner_area = block.inner(center_area);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    frame.render_widget(Clear, center_area);
    if config.background {
        render_background(frame, center_area, config.theme.bg_popup);
    }
    frame.render_widget(block, center_area);
    let items: Vec<ListItem> = ALL_BROWSERS
        .iter()
        .map(|b| ListItem::new(format!(" {:?}", b)))
        .collect();
    let list_widget = List::new(items).highlight_style(config.theme.selected_item());
    frame.render_stateful_widget(list_widget, layout[1], &mut app.browser_liststate);
}

// ACCOUNT
fn render_select_account_popup(frame: &mut Frame, app: &mut App, area: Rect, config: &Config) {
    let PopupState::SelectAccount {
        accounts,
        accounts_liststate,
        browser,
        profile,
        container,
    } = &mut app.popup_state
    else {
        return;
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(config.theme.active_border_style())
        .title(" Account 󰀄 ")
        .title_bottom(select_keymap(&config.theme).centered());

    let center_area = area.centered(Constraint::Percentage(50), Constraint::Length(20));
    let inner_area = block.inner(center_area);
    let hor_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Length(1),
            Constraint::Percentage(30),
        ])
        .split(inner_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(hor_layout[0]);

    frame.render_widget(Clear, center_area);
    if config.background {
        render_background(frame, center_area, config.theme.bg_popup);
    }
    frame.render_widget(block, center_area);
    render_side_info(
        frame,
        hor_layout[2],
        vec![
            format!("  : {:?}", *browser),
            format!("  : {}", profile.name),
            format!(
                "  : {}",
                container
                    .as_ref()
                    .map(|c| c.name.as_str())
                    .unwrap_or("None")
            ),
        ],
    );
    render_v_divider(
        frame,
        hor_layout[1],
        Borders::LEFT,
        config.theme.active_border_style(),
    );
    if accounts.is_empty() {
        let msg = Paragraph::new("No accounts found")
            .style(config.theme.text_style())
            .alignment(Alignment::Center);
        frame.render_widget(msg, layout[1]);
        return;
    }
    let items: Vec<ListItem> = accounts
        .iter()
        .map(|a| ListItem::new(format!(" {}", a.email)))
        .collect();

    let list_widget = List::new(items).highlight_style(config.theme.selected_item());

    frame.render_stateful_widget(list_widget, layout[1], accounts_liststate);
}

// CREATE PLAYLIST
fn render_create_playlist_popup(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    config: &Config,
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
        Span::styled("[ Tab: ", config.theme.text_style()),
        Span::styled("Next ", config.theme.key_style()),
        Span::styled("| Enter: ", config.theme.text_style()),
        Span::styled("Create ", config.theme.key_style()),
        Span::styled("| Esc: ", config.theme.text_style()),
        Span::styled("Cancel ", config.theme.key_style()),
        Span::styled("]", config.theme.text_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(config.theme.active_border_style())
        .title(" Create Playlist ")
        .title_bottom(keymap.centered());

    let center_area = area.centered(Constraint::Percentage(50), Constraint::Length(20));
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
    if config.background {
        render_background(frame, center_area, config.theme.bg_popup);
    }
    frame.render_widget(block, center_area);
    if app.api_loading_kind == Some(ApiLoadingKind::CreatePlaylist) {
        render_spinner(frame, layout[0], &config.theme, start_time);
    }
    render_input_field(
        frame,
        layout[1],
        &config.theme,
        " Title:",
        title,
        matches!(focused_field, CreatePlaylistFocus::Title),
    );
    render_input_field(
        frame,
        layout[2],
        &config.theme,
        " Desc:",
        description,
        matches!(focused_field, CreatePlaylistFocus::Description),
    );
    render_privacy_selector(
        frame,
        layout[3],
        &config.theme,
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
    let line = if is_focused {
        Line::from(vec![Span::raw(text), Span::raw("_")])
    } else {
        Line::from(text)
    };
    let input = Paragraph::new(line)
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
fn render_spinner(f: &mut Frame, area: Rect, theme: &Theme, start_time: std::time::Instant) {
    let spinners = ["", "", "", "", "", ""];
    let elapsed = start_time.elapsed().as_millis();
    let index = ((elapsed / 80) as usize) % spinners.len();
    let [centered_area] = Layout::vertical([Constraint::Length(1)])
        .flex(Flex::Center)
        .areas(area);

    let spinner_widget = Paragraph::new(spinners[index])
        .style(theme.text_style())
        .alignment(Alignment::Center);

    f.render_widget(spinner_widget, centered_area);
}

fn render_background(frame: &mut Frame, area: Rect, color: Color) {
    frame.render_widget(Paragraph::new("").style(Style::default().bg(color)), area);
}
