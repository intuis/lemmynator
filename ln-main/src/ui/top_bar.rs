use std::sync::Arc;

use lemmy_api_common::person::GetUnreadCountResponse;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::Ctx;

use super::components::Component;

pub struct TopBar {
    pub unread_counts: GetUnreadCountResponse,
    ctx: Arc<Ctx>,
}

impl TopBar {
    pub async fn new(ctx: Arc<Ctx>, unread_counts: GetUnreadCountResponse) -> Self {
        Self { ctx, unread_counts }
    }

    fn total_unreads(&self) -> i64 {
        let unread_counts = &self.unread_counts;
        unread_counts.replies + unread_counts.mentions + unread_counts.private_messages
    }

    fn menu_text(&self) -> Line {
        let total_unreads = self.total_unreads();
        let mut spans = vec![];

        spans.push({
            if total_unreads == 0 {
                Span::raw(" 󰂚 ")
            } else {
                Span::styled(
                    format!(" 󱅫 {total_unreads}"),
                    Style::new().fg(self.ctx.config.general.accent_color.as_ratatui()),
                )
            }
        });

        spans.push(Span::raw(format!(
            "   {}  ",
            &self.ctx.config.connection.username
        )));
        Line::from(spans)
    }
}

impl Component for TopBar {
    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let paragraph = Paragraph::new(self.menu_text()).right_aligned();
        f.render_widget(paragraph, rect);

        let paragraph =
            Paragraph::new(format!(" {}", &*self.ctx.config.connection.instance)).left_aligned();
        f.render_widget(paragraph, rect);
    }
}
