use std::sync::Arc;

use crate::{action::Action, app::Ctx};

use super::{
    components::{
        tabs::{CurrentTab, TabComponent},
        Component,
    },
    page::Page,
};

use lemmy_api_common::{lemmy_db_schema::ListingType, person::GetUnreadCountResponse};
use ratatui::{prelude::*, widgets::Paragraph};

pub struct MainWindow {
    posts_viewer: PostsComponent,
    ctx: Arc<Ctx>,
}

impl MainWindow {
    pub async fn new(ctx: Arc<Ctx>) -> Self {
        Self {
            posts_viewer: PostsComponent::new(Arc::clone(&ctx)).await,
            ctx: Arc::clone(&ctx),
        }
    }
}

impl Component for MainWindow {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        self.posts_viewer.handle_actions(action)
    }

    fn render(&mut self, f: &mut Frame, _rect: Rect) {
        self.posts_viewer.render(f, f.size());
    }
}

struct PostsComponent {
    tabs: TabComponent,
    top_bar: TopBar,
    subscribed_page: Page,
    local_page: Page,
    all_page: Page,
    ctx: Arc<Ctx>,
}

impl PostsComponent {
    async fn new(ctx: Arc<Ctx>) -> Self {
        let unread_counts: GetUnreadCountResponse = ctx
            .client
            .get(format!(
                "https://{}/api/v3/user/unread_count",
                ctx.config.connection.instance
            ))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        Self {
            tabs: TabComponent::new(Arc::clone(&ctx)),
            top_bar: TopBar::new(
                Arc::clone(&ctx),
                ctx.config.connection.username.clone(),
                unread_counts,
            )
            .await,
            subscribed_page: Page::new(ListingType::Subscribed, Arc::clone(&ctx)).await,
            local_page: Page::new(ListingType::Local, Arc::clone(&ctx)).await,
            all_page: Page::new(ListingType::All, Arc::clone(&ctx)).await,
            ctx,
        }
    }
}

impl Component for PostsComponent {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::ChangeTab(_) => self.tabs.handle_actions(action),
            _ => match self.tabs.current_tab {
                CurrentTab::Subscribed => self.subscribed_page.handle_actions(action),
                CurrentTab::Local => self.local_page.handle_actions(action),
                CurrentTab::All => self.all_page.handle_actions(action),
            },
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [tabs_rect, main_rect] =
            Layout::vertical([Constraint::Length(1), Constraint::Percentage(100)]).areas(rect);

        let posts_rect = Layout::horizontal([
            Constraint::Percentage(5),
            Constraint::Percentage(90),
            Constraint::Percentage(5),
        ])
        .split(main_rect)[1];

        self.top_bar.render(f, tabs_rect);
        self.tabs.render(f, tabs_rect);

        match self.tabs.current_tab {
            CurrentTab::Subscribed => self.subscribed_page.render(f, posts_rect),
            CurrentTab::Local => self.local_page.render(f, posts_rect),
            CurrentTab::All => self.all_page.render(f, posts_rect),
        }
    }
}

struct TopBar {
    username: String,
    unread_counts: GetUnreadCountResponse,
    ctx: Arc<Ctx>,
}

impl TopBar {
    async fn new(ctx: Arc<Ctx>, username: String, unread_counts: GetUnreadCountResponse) -> Self {
        Self {
            username,
            ctx,
            unread_counts,
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
                Span::raw(" 󰂚 0")
            } else {
                Span::styled(
                    format!(" 󱅫 {total_unreads}"),
                    Style::new().fg(self.ctx.config.general.accent_color.as_ratatui()),
                )
            }
        });

        spans.push(Span::raw(format!("  {} ", &self.username)));
        Line::from(spans)
    }
}

impl Component for TopBar {
    fn handle_actions(&mut self, _action: Action) -> Option<Action> {
        None
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let paragraph = Paragraph::new(self.menu_text()).right_aligned();
        f.render_widget(paragraph, rect);
    }
}
