use std::sync::Arc;

use lemmy_api_common::person::GetUnreadCountResponse;
use ln_config::CONFIG;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::Ctx;

use super::components::{tabs::TabComponent, Component};

pub struct TopBar {
    pub tabs: TabComponent,
    pub unread_counts: GetUnreadCountResponse,
    ctx: Arc<Ctx>,
}

impl TopBar {
    pub async fn new(ctx: Arc<Ctx>, unread_counts: GetUnreadCountResponse) -> Self {
        Self {
            tabs: TabComponent::new(Arc::clone(&ctx)),
            unread_counts,
            ctx,
        }
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
                    Style::new().fg(CONFIG.general.accent_color),
                )
            }
        });

        spans.push(Span::raw(format!("   {}  ", &CONFIG.connection.username)));
        Line::from(spans)
    }
}

impl Component for TopBar {
    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let paragraph = Paragraph::new(self.menu_text()).right_aligned();
        f.render_widget(paragraph, rect);

        let paragraph = Paragraph::new(format!(" {}", &*CONFIG.connection.instance)).left_aligned();
        f.render_widget(paragraph, rect);

        self.tabs.render(f, rect);
    }
}
