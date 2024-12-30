use std::sync::Arc;

use crate::{
    action::{Action, UpdateAction},
    app::Ctx,
};

use super::{
    components::Component, listing::Listing, listing_view::ListingView, post_view::PostView,
    top_bar::TopBar,
};

use anyhow::Result;
use lemmy_api_common::{
    comment::{GetComments, GetCommentsResponse},
    lemmy_db_schema::SortType,
    person::GetUnreadCountResponse,
};
use ratatui::prelude::*;

pub struct MainWindow {
    top_bar: TopBar,
    listing_view: ListingView,
    post_view: Option<PostView>,
    ctx: Arc<Ctx>,
}

impl MainWindow {
    pub async fn new(ctx: Arc<Ctx>) -> Result<Self> {
        Ok(Self {
            top_bar: TopBar::new(
                Arc::clone(&ctx),
                GetUnreadCountResponse {
                    replies: 0,
                    mentions: 0,
                    private_messages: 0,
                },
            )
            .await,
            listing_view: ListingView::new(Arc::clone(&ctx)).await,
            post_view: None,
            ctx,
        })
    }

    fn get_current_listing(&mut self) -> &mut Listing {
        self.listing_view
            .listings
            .get_mut(&self.top_bar.tabs.tabs_state.current())
            .expect("Listings already populated")
    }

    fn change_sort(&mut self) {
        self.top_bar.tabs.change_sort();
        let new_listing = Listing::new(
            self.top_bar.tabs.current_listing_type(),
            self.top_bar.tabs.current_sort(),
            Arc::clone(&self.ctx),
        )
        .unwrap();
        self.listing_view
            .listings
            .insert(self.top_bar.tabs.tabs_state.current(), new_listing)
            .unwrap();

        self.ctx.send_action(Action::Render);
    }
}

impl Component for MainWindow {
    fn handle_actions(&mut self, action: Action) {
        match action {
            _ if self.post_view.is_some() => {
                if let Action::Quit = action {
                    self.post_view = None;
                    self.ctx.send_action(Action::Render);
                    return;
                }

                if let Some(post_view) = &mut self.post_view {
                    post_view.handle_actions(action);
                }
            }
            Action::ChangeSort => self.change_sort(),
            Action::Quit => {
                self.ctx.send_action(Action::ForceQuit);
            }
            Action::ChangeTab(_) => self.top_bar.tabs.handle_actions(action),
            _ => self.get_current_listing().handle_actions(action),
        }
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        match action {
            UpdateAction::UpdateUnreadsCount(unreads_count) => {
                self.top_bar.unread_counts = unreads_count;
                self.ctx.send_action(Action::Render);
            }
            UpdateAction::CommentsForCurrentPost(comments) => {
                if let Some(post_view) = &mut self.post_view {
                    post_view.post.comments = Some(comments.comments.into());
                    self.ctx.send_action(Action::Render);
                }
            }
            UpdateAction::ViewPost(post) => {
                let params = GetComments {
                    community_id: Some(post.community_id),
                    post_id: Some(post.id.clone()),
                    max_depth: Some(8),
                    limit: Some(100),
                    ..Default::default()
                };
                self.post_view = Some(PostView::new(post));
                self.ctx.send_action(Action::Render);

                let _ctx = self.ctx.clone();
                tokio::task::spawn(async move {
                    let res: GetCommentsResponse = _ctx
                        .client
                        .get(format!(
                            "https://{}/api/v3/comment/list",
                            _ctx.config.connection.instance
                        ))
                        .query(&params)
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .unwrap();
                    _ctx.send_update_action(UpdateAction::CommentsForCurrentPost(res));
                });
            }
            _ => self.listing_view.handle_update_action(action),
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [top_bar_rect, rect] =
            Layout::vertical([Constraint::Length(1), Constraint::Percentage(100)]).areas(rect);

        self.top_bar.render(f, top_bar_rect);

        if let Some(post_view) = &mut self.post_view {
            post_view.render(f, rect);
        } else {
            let [_, rect, _] = Layout::horizontal([
                Constraint::Percentage(5),
                Constraint::Percentage(90),
                Constraint::Percentage(5),
            ])
            .areas(rect);
            self.listing_view
                .listings
                .get_mut(&self.top_bar.tabs.tabs_state.current())
                .expect("Listings already populated")
                .render(f, rect);
        }
    }
}
