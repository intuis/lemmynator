use std::sync::Arc;

use crate::{
    action::{Action, UpdateAction},
    app::Ctx,
};

use super::{components::Component, listing_view::ListingView, post_view::PostView};

use anyhow::Result;
use lemmy_api_common::comment::{GetComments, GetCommentsResponse};
use ratatui::prelude::*;

pub struct MainWindow {
    listing_view: ListingView,
    post_view: Option<PostView>,
    ctx: Arc<Ctx>,
}

impl MainWindow {
    pub async fn new(ctx: Arc<Ctx>) -> Result<Self> {
        Ok(Self {
            listing_view: ListingView::new(Arc::clone(&ctx)).await,
            post_view: None,
            ctx,
        })
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
            Action::Quit => {
                self.ctx.send_action(Action::ForceQuit);
            }
            _ => self.listing_view.handle_actions(action),
        }
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        match action {
            UpdateAction::CommentsForCurrentPost(comments) => {
                if let Some(post_view) = &mut self.post_view {
                    post_view.post.comments = Some(comments.comments);
                    self.ctx.send_action(Action::Render);
                }
            }
            UpdateAction::ViewPost(post) => {
                let params = GetComments {
                    community_id: Some(post.community_id),
                    post_id: Some(post.id.clone()),
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
        if let Some(post_view) = &mut self.post_view {
            post_view.render(f, rect);
        } else {
            self.listing_view.render(f, rect);
        }
    }
}
