use std::time::Duration;

use config::Config;
use error::log_to_file;
use ratatui::{Frame, layout::Rect, style::Style};
use ratatui_notifications::{Anchor, AutoDismiss, Level, Notification, Notifications};

pub enum NotifyType {
    Success,
    Error,
}
pub struct NotificationManager {
    inner: Notifications,
    success_style: Style,
    error_style: Style,
}

impl NotificationManager {
    pub fn new(config: &Config) -> Self {
        let (success_style, error_style) = if config.background {
            (
                config.theme.success_style().bg(config.theme.bg_popup),
                config.theme.error_style().bg(config.theme.bg_popup),
            )
        } else {
            (config.theme.success_style(), config.theme.error_style())
        };
        Self {
            inner: Notifications::new(),
            success_style,
            error_style,
        }
    }
    pub fn has_notification(&self) -> bool {
        self.inner.has_notification()
    }
    pub fn tick(&mut self, delta: Duration) {
        self.inner.tick(delta);
    }
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.inner.render(frame, area);
    }
    pub fn notify(&mut self, noti_type: NotifyType, msg: String) {
        let style = match noti_type {
            NotifyType::Success => self.success_style,
            NotifyType::Error => self.error_style,
        };

        if let Ok(notif) = Notification::new(msg)
            .level(Level::Info)
            .anchor(Anchor::TopRight)
            .auto_dismiss(AutoDismiss::After(Duration::from_millis(2500)))
            .max_size(
                ratatui_notifications::SizeConstraint::Percentage(25.0),
                ratatui_notifications::SizeConstraint::Absolute(4),
            )
            .timing(
                ratatui_notifications::Timing::Fixed(Duration::from_millis(50)),
                ratatui_notifications::Timing::Fixed(Duration::from_millis(2400)),
                ratatui_notifications::Timing::Fixed(Duration::from_millis(50)),
            )
            .style(style)
            .border_style(style)
            .build()
        {
            if let Err(e) = self.inner.add(notif) {
                log_to_file(&e);
            }
        }
    }
}
