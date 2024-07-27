use std::{collections::HashMap, sync::Arc};

use crate::{
    action::{Action, UpdateAction},
    app::Ctx,
};

use super::{
    components::{
        tabs::{CurrentTab, TabComponent},
        Component,
    },
    listing::Listing,
};

use anyhow::Result;
use lemmy_api_common::{lemmy_db_schema::SortType, person::GetUnreadCountResponse};
use ratatui::{prelude::*, widgets::Paragraph};

pub struct MainWindow {
    tabs: TabComponent,
    top_bar: TopBar,
    listings: HashMap<CurrentTab, Listing>,
    ctx: Arc<Ctx>,
}

impl MainWindow {
    pub async fn new(ctx: Arc<Ctx>) -> Result<Self> {
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

        let listings = HashMap::new();

        let mut posts_component = Self {
            tabs: TabComponent::new(Arc::clone(&ctx)),
            top_bar: TopBar::new(Arc::clone(&ctx), unread_counts).await,
            listings,
            ctx,
        };

        posts_component.populate_listings();

        Ok(posts_component)
    }

    fn populate_listings(&mut self) {
        let default_sort_type = SortType::Hot;

        for tab in [CurrentTab::Subscribed, CurrentTab::Local, CurrentTab::All] {
            let ctx = Arc::clone(&self.ctx);
            let listing = Listing::new(tab.as_listing_type(), default_sort_type, ctx).unwrap();
            self.listings.insert(tab, listing);
        }
    }

    fn get_current_listing(&mut self) -> &mut Listing {
        self.listings
            .get_mut(&self.tabs.current_tab)
            .expect("Listings already populated")
    }

    fn change_sort(&mut self, sort_type: SortType) {
        self.tabs.change_sort(sort_type);
        let new_listing = Listing::new(
            self.tabs.current_listing_type(),
            self.tabs.current_sort(),
            Arc::clone(&self.ctx),
        )
        .unwrap();
        self.listings
            .insert(self.tabs.current_tab, new_listing)
            .unwrap();

        self.ctx.send_action(Action::Render);
    }
}

impl Component for MainWindow {
    fn handle_actions(&mut self, action: Action) {
        match action {
            Action::ChangeTab(_) => self.tabs.handle_actions(action),
            Action::ChangeSort(sort_type) => self.change_sort(sort_type),
            _ => self.get_current_listing().handle_actions(action),
        }
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        match &action {
            UpdateAction::NewPage(listing_type, _, _) => {
                self.listings
                    .get_mut(&(*listing_type).into())
                    .expect("Listing already populated")
                    .handle_update_action(action);
            }
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
            .expect("Listings already populated")
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
    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let paragraph = Paragraph::new(self.menu_text()).right_aligned();
        f.render_widget(paragraph, rect);

        let paragraph =
            Paragraph::new(format!(" {}", &*self.ctx.config.connection.instance)).left_aligned();
        f.render_widget(paragraph, rect);
    }
}
