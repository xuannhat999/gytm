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
        Self::catppuccin_mocha()
    }
}
impl Theme {
    pub const fn catppuccin_mocha() -> Self {
        Theme {
            primary: Color::Rgb(203, 166, 247),
            secondary: Color::Rgb(166, 227, 161),
            active: Color::Rgb(137, 180, 250),
            inactive: Color::Rgb(108, 112, 134),
            base: Color::Rgb(205, 214, 244),
        }
    }
    pub const fn tokyo_night() -> Self {
        Theme {
            primary: Color::Rgb(187, 154, 247),
            secondary: Color::Rgb(158, 206, 106),
            active: Color::Rgb(122, 162, 247),
            inactive: Color::Rgb(86, 95, 137),
            base: Color::Rgb(169, 177, 214),
        }
    }
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
pub fn success_style() -> Style {
    Style::default().fg(Color::Rgb(166, 227, 161))
}
pub fn error_style() -> Style {
    Style::default().fg(Color::Rgb(243, 139, 168))
}
