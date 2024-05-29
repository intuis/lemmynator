pub mod lemmynator_post;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use lemmy_api_common::{
    lemmy_db_schema::{ListingType, SortType},
    lemmy_db_views::structs::PaginationCursor,
    post::{GetPosts, GetPostsResponse},
};
use ratatui::prelude::*;

use crate::{action::Action, app::Ctx};

use self::lemmynator_post::LemmynatorPost;

use super::components::Component;

pub struct Page {
    listing_type: ListingType,
    next_page: Arc<Mutex<PaginationCursor>>,
    posts: Arc<Mutex<Vec<LemmynatorPost>>>,
    posts_offset: usize,
    currently_focused: u8,
    currently_displaying: u8,
    can_fetch_new_pages: Arc<AtomicBool>,
    ctx: Arc<Ctx>,
}

impl Page {
    pub async fn new(listing_type: ListingType, ctx: Arc<Ctx>) -> Self {
        let local_posts_req = GetPosts {
            type_: Some(listing_type),
            limit: Some(10),
            sort: Some(SortType::Hot),
            ..Default::default()
        };

        let page = ctx
            .client
            .get("https://slrpnk.net/api/v3/post/list")
            .query(&local_posts_req)
            .send()
            .await
            .unwrap();

        let page: GetPostsResponse = page.json().await.unwrap();

        let next_page = page.next_page.unwrap();

        let posts = page
            .posts
            .into_iter()
            .map(|post| LemmynatorPost::from_lemmy_post(post, Arc::clone(&ctx)))
            .collect();

        Self {
            listing_type,
            posts: Arc::new(Mutex::new(posts)),
            next_page: Arc::new(Mutex::new(next_page)),
            posts_offset: 0,
            currently_focused: 0,
            currently_displaying: 0,
            can_fetch_new_pages: Arc::new(AtomicBool::new(true)),
            ctx: Arc::clone(&ctx),
        }
    }

    async fn fetch_next_page(
        cursor: Arc<Mutex<PaginationCursor>>,
        posts: Arc<Mutex<Vec<LemmynatorPost>>>,
        atomic_lock: Arc<AtomicBool>,
        ctx: Arc<Ctx>,
        listing_type: ListingType,
    ) {
        let posts_req = GetPosts {
            type_: Some(listing_type),
            sort: Some(lemmy_api_common::lemmy_db_schema::SortType::Hot),
            page_cursor: Some(cursor.lock().unwrap().clone()),
            limit: Some(20),
            ..Default::default()
        };

        let req = ctx
            .client
            .get("http://slrpnk.net/api/v3/post/list")
            .query(&posts_req);

        let page: GetPostsResponse = req.send().await.unwrap().json().await.unwrap();

        let mut new_posts = page
            .posts
            .into_iter()
            .map(|post| LemmynatorPost::from_lemmy_post(post, Arc::clone(&ctx)))
            .collect();

        posts.lock().unwrap().append(&mut new_posts);
        *cursor.lock().unwrap() = page.next_page.unwrap();
        atomic_lock.store(true, Ordering::SeqCst);
        ctx.action_tx.send(Action::Render).unwrap();
    }

    fn scroll_up(&mut self) {
        if self.currently_focused == 0 && self.posts_offset != 0 {
            self.posts_offset -= self.currently_displaying as usize;
            self.currently_focused = self.currently_displaying - 1;
        } else if self.currently_focused != 0 {
            self.currently_focused -= 1;
        }
    }

    fn scroll_down(&mut self) {
        self.currently_focused += 1;
        if self.currently_focused >= self.currently_displaying {
            self.posts_offset += self.currently_displaying as usize;
            self.currently_focused = 0;
        }
    }

    fn update_count_of_currently_displaying(&mut self, rect: Rect) {
        self.currently_displaying = (rect.height / 8) as u8;
    }
}

impl Component for Page {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::Up => {
                self.scroll_up();
                Some(Action::Render)
            }
            Action::Down => {
                self.scroll_down();
                Some(Action::Render)
            }
            _ => None,
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        self.update_count_of_currently_displaying(rect);
        let blocks_count = rect.height / 8;

        let layouts = Layout::vertical(vec![
            Constraint::Length(8);
            self.currently_displaying as usize
        ])
        .split(rect);

        let mut posts_lock = self.posts.lock().unwrap();

        if posts_lock.len() < self.posts_offset + blocks_count as usize {
            // Can't render a full page. Fetch new pages then and return.
            Self::try_fetch_new_pages(&self);
            return;
        }

        let offseted_posts = &mut posts_lock[self.posts_offset..];

        if let None = offseted_posts.get(2 * blocks_count as usize) {
            // We are getting near the end of available pages, fetch new pages then
            Self::try_fetch_new_pages(&self);
        }

        for index in 0..blocks_count {
            let layout = layouts[index as usize];

            let post = {
                match offseted_posts.get_mut(index as usize) {
                    Some(post) => post,
                    None => {
                        drop(posts_lock);
                        break;
                    }
                }
            };

            if self.currently_focused == index as u8 {
                post.is_focused = true;
            }

            post.render(f, layout);

            post.is_focused = false;
        }
    }
}

impl Page {
    fn try_fetch_new_pages(&self) {
        if let Ok(true) = self.can_fetch_new_pages.compare_exchange(
            true,
            false,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            tokio::task::spawn(Self::fetch_next_page(
                Arc::clone(&self.next_page),
                Arc::clone(&self.posts),
                Arc::clone(&self.can_fetch_new_pages),
                Arc::clone(&self.ctx),
                self.listing_type,
            ));
        }
    }
}
