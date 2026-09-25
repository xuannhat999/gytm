use crossterm::event::{KeyCode, KeyEvent};
use data::{api_client::ALL_BROWSERS, app::PopupState};
use ratatui::widgets::ListState;

use crate::app::App;

pub(crate) fn handle_list_event(key_event: KeyEvent, app: &mut App) {
    let (state, len) = if matches!(app.popup_state, PopupState::SaveSong { .. }) {
        (&mut app.cus_playlists_liststate, app.cus_playlists.len())
    } else if matches!(app.popup_state, PopupState::SelectBrowser) {
        (&mut app.browser_liststate, ALL_BROWSERS.len())
    } else if let PopupState::SelectBrowserProfile {
        profiles,
        profiles_liststate,
        ..
    } = &mut app.popup_state
    {
        (profiles_liststate, profiles.len())
    } else if let PopupState::SelectGeckoContainer {
        containers,
        containers_liststate,
        ..
    } = &mut app.popup_state
    {
        (containers_liststate, containers.len())
    } else if let PopupState::SelectAccount {
        accounts,
        accounts_liststate,
        ..
    } = &mut app.popup_state
    {
        (accounts_liststate, accounts.len())
    } else {
        (&mut ListState::default(), 0)
    };
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => App::next_item(state, len),
        KeyCode::Up | KeyCode::Char('k') => App::previous_item(state, len),
        _ => {}
    }
}
