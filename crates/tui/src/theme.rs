use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub active: Color,
    pub inactive: Color,
    pub base: Color,
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
impl Theme {
    pub fn text_style(&self) -> Style {
        Style::default().fg(self.base)
    }

    pub fn key_style(&self) -> Style {
        Style::default()
            .fg(self.secondary)
            .add_modifier(Modifier::BOLD)
    }
    pub fn active_border_style(&self) -> Style {
        Style::default()
            .fg(self.active)
            .add_modifier(Modifier::BOLD)
    }
    pub fn inactive_border_style(&self) -> Style {
        Style::default().fg(self.inactive)
    }
    pub fn selected_item(&self) -> Style {
        Style::default().bg(Color::Rgb(69, 71, 90)).fg(self.primary)
    }
}
