use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub active: Color,
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
            primary: Color::Rgb(203, 166, 247),
            secondary: Color::Rgb(249, 226, 175),
            active: Color::Rgb(137, 180, 250),
            inactive: Color::Rgb(108, 112, 134),
            base: Color::Rgb(205, 214, 244),
            bg: Color::Rgb(30, 30, 46),
            bg_popup: Color::Rgb(40, 40, 56),
            surface: Color::Rgb(69, 71, 90),
        }
    }

    pub const fn gruvbox() -> Self {
        Theme {
            primary: Color::Rgb(184, 187, 38),
            secondary: Color::Rgb(215, 153, 33),
            active: Color::Rgb(222, 130, 50),
            inactive: Color::Rgb(168, 152, 131),
            base: Color::Rgb(235, 219, 178),
            bg: Color::Rgb(40, 40, 40),
            bg_popup: Color::Rgb(50, 50, 50),
            surface: Color::Rgb(60, 56, 54),
        }
    }

    pub const fn dracula() -> Self {
        Theme {
            primary: Color::Rgb(189, 147, 249),
            secondary: Color::Rgb(241, 250, 140),
            active: Color::Rgb(139, 233, 253),
            inactive: Color::Rgb(98, 114, 164),
            base: Color::Rgb(248, 248, 242),
            bg: Color::Rgb(40, 42, 54),
            bg_popup: Color::Rgb(50, 52, 64),
            surface: Color::Rgb(68, 71, 90),
        }
    }

    pub const fn tokyo_night() -> Self {
        Theme {
            primary: Color::Rgb(187, 154, 247),
            secondary: Color::Rgb(224, 175, 104),
            active: Color::Rgb(122, 162, 247),
            inactive: Color::Rgb(86, 95, 137),
            base: Color::Rgb(169, 177, 214),
            bg: Color::Rgb(26, 27, 38),
            bg_popup: Color::Rgb(36, 37, 48),
            surface: Color::Rgb(41, 46, 66),
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "tokyo_night" => Self::tokyo_night(),
            "gruvbox" => Self::gruvbox(),
            "dracula" => Self::dracula(),
            _ => Self::catppuccin_mocha(),
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
        Style::default().bg(self.surface).fg(self.primary)
    }
}
