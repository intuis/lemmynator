use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use crate::{action::Action, app::Ctx};

use super::{
    components::{
        tabs::{CurrentTab, TabComponent},
        Component,
    },
    listing::Listing,
};

use anyhow::Result;
use lemmy_api_common::{
    lemmy_db_schema::{ListingType, SortType},
    person::GetUnreadCountResponse,
};
use ratatui::{prelude::*, widgets::Paragraph};

pub struct MainWindow {
    posts_viewer: PostsComponent,
}

impl MainWindow {
    pub async fn new(ctx: Arc<Ctx>) -> Result<Self> {
        Ok(Self {
            posts_viewer: PostsComponent::new(Arc::clone(&ctx)).await?,
        })
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

// TODO: make this struct a MainWindow later
struct PostsComponent {
    tabs: TabComponent,
    top_bar: TopBar,
    listings: HashMap<CurrentTab, Listing>,
    ctx: Arc<Ctx>,
}

impl PostsComponent {
    async fn new(ctx: Arc<Ctx>) -> Result<Self> {
        let unread_counts: GetUnreadCountResponse = ctx
            .client
            .get(format!(
                "https://{}/api/v3/user/unread_count",
                ctx.config.connection.instance
            ))
            .send()
            .await?
            .json()
            .await?;

        let default_sort_type = SortType::Hot;

        let mut listings = HashMap::new();

        listings.insert(
            CurrentTab::Subscribed,
            Listing::new(ListingType::Subscribed, default_sort_type, Arc::clone(&ctx)).unwrap(),
        );

        listings.insert(
            CurrentTab::Local,
            Listing::new(ListingType::Local, default_sort_type, Arc::clone(&ctx)).unwrap(),
        );

        listings.insert(
            CurrentTab::All,
            Listing::new(ListingType::All, default_sort_type, Arc::clone(&ctx)).unwrap(),
        );

        Ok(Self {
            tabs: TabComponent::new(Arc::clone(&ctx)),
            top_bar: TopBar::new(Arc::clone(&ctx), unread_counts).await,
            listings,
            ctx,
        })
    }
}

impl Component for PostsComponent {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::ChangeTab(_) => self.tabs.handle_actions(action),
            Action::ChangeSort(_) => {
                self.tabs.handle_actions(action);
                self.listings.insert(
                    self.tabs.current_tab,
                    Listing::new(
                        self.tabs.current_tab.as_listing_type(),
                        *self.tabs.sort_hash.get(&self.tabs.current_tab).unwrap(),
                        Arc::clone(&self.ctx),
                    )
                    .unwrap(),
                );

                Some(Action::Render)
            }
            _ => self
                .listings
                .get_mut(&self.tabs.current_tab)
                .unwrap()
                .handle_actions(action),
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [tabs_rect, main_rect] =
            Layout::vertical([Constraint::Length(2), Constraint::Percentage(100)]).areas(rect);

        let posts_rect = Layout::horizontal([
            Constraint::Percentage(5),
            Constraint::Percentage(90),
            Constraint::Percentage(5),
        ])
        .split(main_rect)[1];

        self.top_bar.render(f, tabs_rect);
        self.tabs.render(f, tabs_rect);

        self.listings
            .get_mut(&self.tabs.current_tab)
            .unwrap()
            .render(f, posts_rect);
    }
}

struct TopBar {
    unread_counts: GetUnreadCountResponse,
    ctx: Arc<Ctx>,
}

impl TopBar {
    async fn new(ctx: Arc<Ctx>, unread_counts: GetUnreadCountResponse) -> Self {
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
                Span::raw(" 󰂚 0")
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
    fn handle_actions(&mut self, _action: Action) -> Option<Action> {
        None
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let paragraph = Paragraph::new(self.menu_text()).right_aligned();
        f.render_widget(paragraph, rect);

        let paragraph =
            Paragraph::new(format!(" {}", &*self.ctx.config.connection.instance)).left_aligned();
        f.render_widget(paragraph, rect);
    }
}
