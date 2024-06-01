pub mod lemmynator_post;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use anyhow::Result;
use lemmy_api_common::{
    lemmy_db_schema::ListingType,
    lemmy_db_views::structs::PaginationCursor,
    post::{GetPosts, GetPostsResponse},
};
use ratatui::{prelude::*, widgets::Paragraph};

use self::lemmynator_post::LemmynatorPost;
use super::{centered_rect, components::Component};
use crate::{action::Action, app::Ctx};

pub struct Page {
    listing_type: ListingType,
    page_data: Arc<Mutex<Option<PageData>>>,
    posts_offset: usize,
    currently_focused: u8,
    currently_displaying: u8,
    can_fetch_new_pages: Arc<AtomicBool>,
    ctx: Arc<Ctx>,
}

struct PageData {
    next_page: PaginationCursor,
    posts: Vec<LemmynatorPost>,
}

impl PageData {
    fn new(posts: Vec<LemmynatorPost>, next_page: PaginationCursor) -> Self {
        PageData { posts, next_page }
    }
}

impl Page {
    pub async fn new(listing_type: ListingType, ctx: Arc<Ctx>) -> Result<Self> {
        Ok(Self {
            listing_type,
            page_data: Arc::new(Mutex::new(None)),
            posts_offset: 0,
            currently_focused: 0,
            currently_displaying: 0,
            can_fetch_new_pages: Arc::new(AtomicBool::new(true)),
            ctx: Arc::clone(&ctx),
        })
    }

    async fn fetch_next_page(
        page_data: Arc<Mutex<Option<PageData>>>,
        atomic_lock: Arc<AtomicBool>,
        ctx: Arc<Ctx>,
        listing_type: ListingType,
    ) {
        let cursor = {
            let page_data_lock = &mut *page_data.lock().unwrap();

            if let Some(page_data) = page_data_lock {
                Some(page_data.next_page.clone())
            } else {
                None
            }
        };

        let posts_req = GetPosts {
            type_: Some(listing_type),
            sort: Some(lemmy_api_common::lemmy_db_schema::SortType::Hot),
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
        if let Some(page_data) = page_data_lock {
            page_data.posts.append(&mut new_posts);
            page_data.next_page = page.next_page.unwrap();
        } else {
            *page_data_lock = Some(PageData::new(new_posts, page.next_page.unwrap()));
        }

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

    fn render_loading_screen(&mut self, f: &mut Frame, rect: Rect) {
        let loading_rect = centered_rect(rect, 50, 1);
        let loading_paragraph = Paragraph::new("").alignment(Alignment::Center);
        f.render_widget(loading_paragraph, loading_rect);
    }

    fn render_posts(&mut self, f: &mut Frame, rect: Rect) {
        self.update_count_of_currently_displaying(rect);

        let page_data = &mut self.page_data.lock().unwrap();
        let page_data = page_data.as_mut().unwrap();

        let main_rect = rect;
        let mut size_occupied = 0;
        let mut rect_pool = rect;
        let mut rects = vec![];

        let current_page = self.posts_offset / self.currently_displaying as usize;
        if current_page > 3 {
            page_data
                .posts
                .drain(0..2 * self.currently_displaying as usize);
            self.posts_offset -= self.currently_displaying as usize * 2;
        }

        if page_data.posts.len() < self.posts_offset + self.currently_displaying as usize {
            // Can't render a full page. Fetch new pages and return.
            Self::try_fetch_new_pages(self);
            return;
        }

        let offseted_posts = &mut page_data.posts[self.posts_offset..];

        if offseted_posts
            .get(2 * self.currently_displaying as usize)
            .is_none()
        {
            // We are getting near the end of available pages, fetch new pages then
            Self::try_fetch_new_pages(self);
        }

        let posts = &mut offseted_posts[..self.currently_displaying as usize];

        for (index, post) in posts.iter_mut().enumerate() {
            let vertical_length = {
                if post.body.is_empty() && !post.is_image_only() {
                    size_occupied += 5;
                    5
                } else if let Some(image_is_wide) = post.image_is_wide() {
                    if image_is_wide {
                        size_occupied += 7;
                        7
                    } else {
                        size_occupied += 8;
                        8
                    }
                } else {
                    size_occupied += 7;
                    7
                }
            };
            let layout = Layout::vertical(vec![
                Constraint::Length(vertical_length),
                Constraint::Percentage(100),
            ])
            .split(rect_pool);

            rects.push(layout[0]);
            rect_pool = layout[1];

            if self.currently_focused == index as u8 {
                post.is_focused = true;
            }
        }

        let mut current_offset = 0;
        for (post, mut rect) in posts.iter_mut().zip(rects.into_iter()) {
            if main_rect.height - size_occupied > self.currently_displaying as u16 {
                current_offset += 1;
                rect.y += current_offset;
            }

            post.render(f, rect);

            post.is_focused = false;
        }
    }

    fn render_bottom_bar(&mut self, f: &mut Frame, rect: Rect) {
        if self.currently_displaying != 0 {
            let current_page_paragraph = Paragraph::new(format!(
                "{} / ",
                (self.posts_offset / self.currently_displaying as usize) + 1,
            ))
            .alignment(Alignment::Center);
            f.render_widget(current_page_paragraph, rect);
        }
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
        let [posts_rect, bottom_bar_rect] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Length(1)]).areas(rect);

        let mut pages_available;
        if let Some(page_data) = &*self.page_data.lock().unwrap() {
            pages_available = true;
            if page_data.posts.len() < self.posts_offset + self.currently_displaying as usize {
                Self::try_fetch_new_pages(self);
                pages_available = false;
            }
        } else {
            pages_available = false;
        }

        if pages_available {
            self.render_posts(f, posts_rect)
        } else {
            self.try_fetch_new_pages();
            self.render_loading_screen(f, posts_rect);
        }

        self.render_bottom_bar(f, bottom_bar_rect)
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
                Arc::clone(&self.page_data),
                Arc::clone(&self.can_fetch_new_pages),
                Arc::clone(&self.ctx),
                self.listing_type,
            ));
        }
    }
}
