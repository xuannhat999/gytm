use crossterm::event::KeyCode;
use ratatui::widgets::ListState;

pub(crate) fn handle_list_event(
    list_state: &mut ListState,
    rows: usize,
    key_code: KeyCode,
) -> bool {
    match key_code {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(i) = list_state.selected()
                && i + 1 < rows
            {
                list_state.select_next();
            }
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(i) = list_state.selected()
                && i > 0
            {
                list_state.select_previous();
            }
            true
        }
        _ => false,
    }
}
