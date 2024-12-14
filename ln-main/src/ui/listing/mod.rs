pub mod lemmynator_post;
mod page;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;
use lemmy_api_common::{
    lemmy_db_schema::{ListingType, SortType},
    lemmy_db_views::structs::PaginationCursor,
    post::{GetPosts, GetPostsResponse},
};
use lemmynator_post::LemmynatorPost;
use ratatui::{prelude::*, widgets::Paragraph};

use self::page::Page;
use super::{centered_rect, components::Component};
use crate::{
    action::{Action, UpdateAction},
    app::Ctx,
};

pub struct Listing {
    listing_type: ListingType,
    pub sort_type: SortType,
    pub page_data: Page,
    pub can_fetch_new_pages: Arc<AtomicBool>,
    ctx: Arc<Ctx>,
}

impl Listing {
    pub fn new(listing_type: ListingType, sort_type: SortType, ctx: Arc<Ctx>) -> Result<Self> {
        Ok(Self {
            listing_type,
            sort_type,
            page_data: Page::new(Arc::clone(&ctx)),
            can_fetch_new_pages: Arc::new(AtomicBool::new(true)),
            ctx: Arc::clone(&ctx),
        })
    }

    pub fn try_fetch_new_pages(&self) {
        if let Ok(true) = self.can_fetch_new_pages.compare_exchange(
            true,
            false,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            tokio::task::spawn(Self::fetch_next_page(
                self.page_data.next_page.clone(),
                self.sort_type,
                Arc::clone(&self.ctx),
                self.listing_type,
            ));
        }
    }

    async fn fetch_next_page(
        page_cursor: Option<PaginationCursor>,
        sort_type: SortType,
        ctx: Arc<Ctx>,
        listing_type: ListingType,
    ) {
        let posts_req = GetPosts {
            type_: Some(listing_type),
            sort: Some(sort_type),
            page_cursor,
            limit: Some(20),
            ..Default::default()
        };

        let req = ctx
            .client
            .get(format!(
                "https://{}/api/v3/post/list",
                ctx.config.connection.instance
            ))
            .query(&posts_req);

        let new_page: GetPostsResponse = req.send().await.unwrap().json().await.unwrap();

        ctx.send_update_action(UpdateAction::NewPage(listing_type, sort_type, new_page));

        ctx.action_tx.send(Action::Render).unwrap();
    }

    // TODO: make this into a component
    fn render_loading_screen(&mut self, f: &mut Frame, rect: Rect) {
        let loading_rect = centered_rect(rect, 50, 1);
        let loading_paragraph = Paragraph::new("").alignment(Alignment::Center);
        f.render_widget(loading_paragraph, loading_rect);
    }
}

impl Component for Listing {
    fn handle_actions(&mut self, action: Action) {
        self.page_data.handle_actions(action);
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        match action {
            UpdateAction::NewPage(_, sort_type, new_page) => {
                if self.sort_type == sort_type {
                    let mut new_posts: Vec<LemmynatorPost> = new_page
                        .posts
                        .into_iter()
                        .map(|post| LemmynatorPost::from_lemmy_post(post, self.ctx.clone()))
                        .collect();

                    self.page_data.all_posts_count += new_posts.len();
                    self.page_data.posts.append(&mut new_posts);
                    self.page_data.next_page = new_page.next_page;
                    self.can_fetch_new_pages.store(true, Ordering::SeqCst);
                }
            }
            _ => unreachable!(),
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [posts_rect, bottom_bar_rect] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Length(1)]).areas(rect);

        let mut are_there_pages_available;

        {
            if !self.page_data.posts.is_empty() {
                are_there_pages_available = true;

                if self.page_data.posts.len()
                    < self.page_data.posts_offset + self.page_data.currently_displaying as usize
                {
                    self.try_fetch_new_pages();
                    are_there_pages_available = false;
                } else if self.page_data.posts.len()
                    < self.page_data.posts_offset + self.page_data.currently_displaying as usize * 2
                {
                    self.try_fetch_new_pages();
                }
            } else {
                are_there_pages_available = false;
            }
        }

        if are_there_pages_available {
            self.page_data.render(f, posts_rect)
        } else {
            self.try_fetch_new_pages();
            self.render_loading_screen(f, posts_rect);
        }

        self.page_data.render_bottom_bar(f, bottom_bar_rect);
    }
}
