use crossterm::event::KeyCode;
use ratatui::widgets::TableState;

pub(crate) fn handle_table_event(table_state: &mut TableState, key_code: KeyCode) -> bool {
    match key_code {
        KeyCode::Char('j') | KeyCode::Down => {
            table_state.select_next();
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            table_state.select_previous();
            true
        }
        _ => false,
    }
}
