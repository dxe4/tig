use ratatui::{Frame, layout::Rect, style::Style, widgets::Paragraph};

use crate::app::App;
use crate::ui::theme::Theme;

pub fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let text = app.status_text();
    let max_len = area.width as usize;
    let text = if text.len() > max_len {
        &text[..max_len]
    } else {
        &text
    };
    let paragraph =
        Paragraph::new(text).style(Style::default().bg(Theme::SURFACE).fg(Theme::SUBTEXT));
    f.render_widget(paragraph, area);
}
