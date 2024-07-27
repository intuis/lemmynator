pub mod lemmynator_post;
mod page;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use anyhow::Result;
use lemmy_api_common::{
    lemmy_db_schema::{ListingType, SortType},
    post::{GetPosts, GetPostsResponse},
};
use ratatui::{prelude::*, widgets::Paragraph};

use self::{lemmynator_post::LemmynatorPost, page::Page};
use super::{centered_rect, components::Component};
use crate::{action::Action, app::Ctx};

pub struct Listing {
    listing_type: ListingType,
    pub sort_type: SortType,
    pub page_data: Arc<Mutex<Page>>,
    pub can_fetch_new_pages: Arc<AtomicBool>,
    ctx: Arc<Ctx>,
}

impl Listing {
    pub fn new(listing_type: ListingType, sort_type: SortType, ctx: Arc<Ctx>) -> Result<Self> {
        Ok(Self {
            listing_type,
            sort_type,
            page_data: Arc::new(Mutex::new(Page::new())),
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
                Arc::clone(&self.page_data),
                Arc::clone(&self.can_fetch_new_pages),
                self.sort_type,
                Arc::clone(&self.ctx),
                self.listing_type,
            ));
        }
    }

    async fn fetch_next_page(
        page_data: Arc<Mutex<Page>>,
        atomic_lock: Arc<AtomicBool>,
        sort_type: SortType,
        ctx: Arc<Ctx>,
        listing_type: ListingType,
    ) {
        let cursor = {
            let page_data_lock = &mut *page_data.lock().unwrap();
            page_data_lock.next_page.clone()
        };

        let posts_req = GetPosts {
            type_: Some(listing_type),
            sort: Some(sort_type),
            page_cursor: cursor,
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

        let page: GetPostsResponse = req.send().await.unwrap().json().await.unwrap();

        let mut new_posts = page
            .posts
            .into_iter()
            .map(|post| LemmynatorPost::from_lemmy_post(post, Arc::clone(&ctx)))
            .collect();

        let page_data_lock = &mut *page_data.lock().unwrap();
        page_data_lock.posts.append(&mut new_posts);
        page_data_lock.next_page = Some(page.next_page.unwrap());

        atomic_lock.store(true, Ordering::SeqCst);
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
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        self.page_data.lock().unwrap().handle_actions(action)
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [posts_rect, bottom_bar_rect] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Length(1)]).areas(rect);

        let mut are_there_pages_available;

        {
            if !self.page_data.lock().unwrap().posts.is_empty() {
                are_there_pages_available = true;

                let page_data_lock = self.page_data.lock().unwrap();
                if page_data_lock.posts.len()
                    < page_data_lock.posts_offset + page_data_lock.currently_displaying as usize
                {
                    self.try_fetch_new_pages();
                    are_there_pages_available = false;
                } else if page_data_lock.posts.len()
                    < page_data_lock.posts_offset + page_data_lock.currently_displaying as usize * 2
                {
                    self.try_fetch_new_pages();
                }
            } else {
                are_there_pages_available = false;
            }
        }

        if are_there_pages_available {
            self.page_data.lock().unwrap().render(f, posts_rect)
        } else {
            self.try_fetch_new_pages();
            self.render_loading_screen(f, posts_rect);
        }

        self.page_data
            .lock()
            .unwrap()
            .render_bottom_bar(f, bottom_bar_rect);
    }
}
