use std::{collections::HashMap, sync::Arc, time::Duration};

use lemmy_api_common::{lemmy_db_schema::SortType, person::GetUnreadCountResponse};

use crate::{
    action::{Action, UpdateAction},
    app::{self, Ctx},
};

use super::{
    components::{tabs::CurrentTab, Component},
    listing::Listing,
};

pub struct ListingView {
    pub listings: HashMap<CurrentTab, Listing>,
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
}

impl Component for ListingView {
    fn handle_actions(&mut self, action: Action) {}

    fn handle_update_action(&mut self, action: crate::action::UpdateAction) {
        match action {
            UpdateAction::NewPage(listing_type, _, _) => {
                self.listings
                    .get_mut(&listing_type.into())
                    .expect("Listing already populated")
                    .handle_update_action(action);
            }
            _ => (),
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {}
}
