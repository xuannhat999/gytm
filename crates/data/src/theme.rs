use ratatui::style::{Color, Style};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation};

#[derive(Debug, Clone)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub third: Color,
    pub inactive: Color,
    pub base: Color,
    pub bg: Color,
    pub surface: Color,
    pub bg_popup: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

impl Theme {
    pub const fn catppuccin_mocha() -> Self {
        Theme {
            primary: Color::Rgb(137, 180, 250),
            secondary: Color::Rgb(44, 53, 68),
            third: Color::Rgb(249, 226, 175),
            inactive: Color::Rgb(108, 112, 134),
            base: Color::Rgb(197, 205, 234),
            bg: Color::Rgb(30, 30, 46),
            bg_popup: Color::Rgb(40, 40, 56),
            surface: Color::Rgb(69, 71, 90),
        }
    }

    pub const fn gruvbox() -> Self {
        Theme {
            primary: Color::Rgb(222, 130, 50),
            secondary: Color::Rgb(65, 52, 40),
            third: Color::Rgb(179, 182, 62),
            inactive: Color::Rgb(134, 121, 104),
            base: Color::Rgb(235, 219, 178),
            bg: Color::Rgb(40, 40, 40),
            bg_popup: Color::Rgb(50, 50, 50),
            surface: Color::Rgb(60, 56, 54),
        }
    }

    pub const fn dracula() -> Self {
        Theme {
            primary: Color::Rgb(139, 233, 253),
            secondary: Color::Rgb(45, 67, 72),
            third: Color::Rgb(241, 250, 140),
            inactive: Color::Rgb(98, 114, 164),
            base: Color::Rgb(248, 248, 242),
            bg: Color::Rgb(40, 42, 54),
            bg_popup: Color::Rgb(50, 52, 64),
            surface: Color::Rgb(68, 71, 90),
        }
    }

    pub const fn tokyo_night() -> Self {
        Theme {
            primary: Color::Rgb(122, 162, 247),
            secondary: Color::Rgb(36, 48, 74),
            third: Color::Rgb(224, 175, 104),
            inactive: Color::Rgb(86, 95, 137),
            base: Color::Rgb(169, 177, 214),
            bg: Color::Rgb(26, 27, 38),
            bg_popup: Color::Rgb(36, 37, 48),
            surface: Color::Rgb(41, 46, 66),
        }
    }
    pub const fn nord() -> Self {
        Theme {
            primary: Color::Rgb(149, 205, 204),
            secondary: Color::Rgb(49, 66, 66),
            third: Color::Rgb(235, 203, 139),
            inactive: Color::Rgb(85, 96, 118),
            base: Color::Rgb(189, 202, 228),
            bg: Color::Rgb(36, 42, 54),
            bg_popup: Color::Rgb(59, 66, 82),
            surface: Color::Rgb(67, 76, 94),
        }
    }
    pub fn from_name(name: &str) -> Self {
        match name {
            "tokyo_night" => Self::tokyo_night(),
            "gruvbox" => Self::gruvbox(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            _ => Self::catppuccin_mocha(),
        }
    }

    pub fn text_style(&self) -> Style {
        Style::default().fg(self.base)
    }

    pub fn key_style(&self) -> Style {
        Style::default().fg(self.third)
    }

    pub fn active_border_style(&self) -> Style {
        Style::default().fg(self.primary).bold()
    }

    pub fn inactive_border_style(&self) -> Style {
        Style::default().fg(self.inactive)
    }

    pub fn selected_item(&self) -> Style {
        Style::default().bg(self.surface).fg(Color::White).bold()
    }
    pub fn table_header_style(&self) -> Style {
        Style::default().bg(self.secondary).fg(self.third).bold()
    }

    pub fn scrollbar(&self) -> Scrollbar<'static> {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .track_style(Style::default().fg(self.secondary))
            .thumb_style(Style::default().fg(self.third))
            .begin_style(Style::default().fg(self.third))
            .end_style(Style::default().fg(self.third))
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(Color::Rgb(243, 139, 168))
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(Color::Rgb(166, 227, 161))
    }
}
