use std::{collections::HashMap, sync::Arc, time::Duration};

use lemmy_api_common::{lemmy_db_schema::SortType, person::GetUnreadCountResponse};
use ratatui::layout::{Constraint, Layout};

use crate::{
    action::{Action, UpdateAction},
    app::{self, Ctx},
};

use super::{
    components::{
        tabs::{CurrentTab, TabComponent},
        Component,
    },
    listing::Listing,
    top_bar::TopBar,
};

pub struct ListingView {
    tabs: TabComponent,
    top_bar: TopBar,
    listings: HashMap<CurrentTab, Listing>,
    ctx: Arc<Ctx>,
}

impl ListingView {
    pub async fn new(ctx: Arc<app::Ctx>) -> Self {
        let _ctx = Arc::clone(&ctx);

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let unread_counts: GetUnreadCountResponse = _ctx
                .client
                .get(format!(
                    "https://{}/api/v3/user/unread_count",
                    _ctx.config.connection.instance
                ))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            _ctx.send_update_action(UpdateAction::UpdateUnreadsCount(unread_counts));
        });
        let mut listing_view = Self {
            tabs: TabComponent::new(Arc::clone(&ctx)),
            top_bar: TopBar::new(
                Arc::clone(&ctx),
                GetUnreadCountResponse {
                    replies: 0,
                    mentions: 0,
                    private_messages: 0,
                },
            )
            .await,
            listings: HashMap::new(),
            ctx,
        };

        listing_view.populate_listings();

        listing_view
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
            .get_mut(&self.tabs.tabs_state.current())
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
            .insert(self.tabs.tabs_state.current(), new_listing)
            .unwrap();

        self.ctx.send_action(Action::Render);
    }
}

impl Component for ListingView {
    fn handle_actions(&mut self, action: Action) {
        match action {
            Action::ChangeTab(_) => self.tabs.handle_actions(action),
            Action::ChangeSort(sort_type) => self.change_sort(sort_type),
            _ => self.get_current_listing().handle_actions(action),
        }
    }

    fn handle_update_action(&mut self, action: crate::action::UpdateAction) {
        match action {
            UpdateAction::NewPage(listing_type, _, _) => {
                self.listings
                    .get_mut(&listing_type.into())
                    .expect("Listing already populated")
                    .handle_update_action(action);
            }
            UpdateAction::UpdateUnreadsCount(unreads_count) => {
                self.top_bar.unread_counts = unreads_count;
                self.ctx.send_action(Action::Render);
            }
            _ => (),
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {
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

        self.listings
            .get_mut(&self.tabs.tabs_state.current())
            .expect("Listings already populated")
            .render(f, posts_rect);
    }
}
