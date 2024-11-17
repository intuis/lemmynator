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
    listing::{lemmynator_post::LemmynatorPost, Listing},
    top_bar::TopBar,
};

use anyhow::Result;
use lemmy_api_common::{lemmy_db_schema::SortType, person::GetUnreadCountResponse};
use ratatui::prelude::*;

pub struct MainWindow {
    tabs: TabComponent,
    top_bar: TopBar,
    listings: HashMap<CurrentTab, Listing>,
    currently_viewing: Option<LemmynatorPost>,
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
            currently_viewing: None,
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

impl Component for MainWindow {
    fn handle_actions(&mut self, action: Action) {
        match action {
            Action::ChangeTab(_) => self.tabs.handle_actions(action),
            Action::ChangeSort(sort_type) => self.change_sort(sort_type),
            _ => self.get_current_listing().handle_actions(action),
        }
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        match action {
            UpdateAction::NewPage(listing_type, _, _) => {
                self.listings
                    .get_mut(&listing_type.into())
                    .expect("Listing already populated")
                    .handle_update_action(action);
            }
            UpdateAction::ViewPost(post) => self.currently_viewing = Some(post),
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

        if let Some(post) = &mut self.currently_viewing {
            post.render(f, posts_rect)
        } else {
            self.listings
                .get_mut(&self.tabs.tabs_state.current())
                .expect("Listings already populated")
                .render(f, posts_rect);
        }
    }
}
