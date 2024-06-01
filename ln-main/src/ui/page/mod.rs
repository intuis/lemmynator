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
    page_data: Arc<Mutex<PageData>>,
    can_fetch_new_pages: Arc<AtomicBool>,
    ctx: Arc<Ctx>,
}

struct PageData {
    posts: Vec<LemmynatorPost>,
    next_page: Option<PaginationCursor>,
    posts_offset: usize,
    currently_focused: u8,
    currently_displaying: u8,
}

impl PageData {
    fn new() -> Self {
        PageData {
            posts: vec![],
            next_page: None,
            posts_offset: 0,
            currently_focused: 0,
            currently_displaying: 0,
        }
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

    fn rects_for_posts(&mut self, mut rect_pool: Rect) -> Vec<Rect> {
        let offseted_posts = &mut self.posts[self.posts_offset..];
        let posts = &mut offseted_posts[..self.currently_displaying as usize];

        let mut rects = vec![];
        for post in posts {
            let vertical_length = {
                if post.body.is_empty() && !post.is_image_only() {
                    5
                } else if let Some(image_is_wide) = post.image_is_wide() {
                    if image_is_wide {
                        7
                    } else {
                        8
                    }
                } else {
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
        }

        rects
    }

    fn render_posts_in_layout(
        &mut self,
        f: &mut Frame,
        rects: &mut [Rect],
        space_for_padding_available: bool,
    ) {
        let offseted_posts = &mut self.posts[self.posts_offset..];
        let posts = &mut offseted_posts[..self.currently_displaying as usize];

        let mut current_offset = 0;
        let mut index = 0;
        for (post, rect) in posts.iter_mut().zip(rects.into_iter()) {
            if space_for_padding_available {
                current_offset += 1;
                rect.y += current_offset;
            }

            if self.currently_focused == index as u8 {
                post.is_focused = true;
            }

            post.render(f, *rect);

            post.is_focused = false;
            index += 1;
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

impl Page {
    pub async fn new(listing_type: ListingType, ctx: Arc<Ctx>) -> Result<Self> {
        Ok(Self {
            listing_type,
            page_data: Arc::new(Mutex::new(PageData::new())),
            can_fetch_new_pages: Arc::new(AtomicBool::new(true)),
            ctx: Arc::clone(&ctx),
        })
    }

    async fn fetch_next_page(
        page_data: Arc<Mutex<PageData>>,
        atomic_lock: Arc<AtomicBool>,
        ctx: Arc<Ctx>,
        listing_type: ListingType,
    ) {
        let cursor = {
            let page_data_lock = &mut *page_data.lock().unwrap();
            page_data_lock.next_page.clone()
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
        page_data_lock.posts.append(&mut new_posts);
        page_data_lock.next_page = Some(page.next_page.unwrap());

        atomic_lock.store(true, Ordering::SeqCst);
        ctx.action_tx.send(Action::Render).unwrap();
    }

    fn render_loading_screen(&mut self, f: &mut Frame, rect: Rect) {
        let loading_rect = centered_rect(rect, 50, 1);
        let loading_paragraph = Paragraph::new("").alignment(Alignment::Center);
        f.render_widget(loading_paragraph, loading_rect);
    }
}

impl Component for Page {
    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        self.page_data.lock().unwrap().handle_actions(action)
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [posts_rect, bottom_bar_rect] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Length(1)]).areas(rect);

        let mut pages_available;

        {
            if !self.page_data.lock().unwrap().posts.is_empty() {
                pages_available = true;

                let page_data_lock = self.page_data.lock().unwrap();
                if page_data_lock.posts.len()
                    < page_data_lock.posts_offset + page_data_lock.currently_displaying as usize
                {
                    self.try_fetch_new_pages();
                    pages_available = false;
                } else if page_data_lock.posts.len()
                    < page_data_lock.posts_offset + page_data_lock.currently_displaying as usize * 2
                {
                    self.try_fetch_new_pages();
                }
            } else {
                pages_available = false;
            }
        }

        if pages_available {
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

impl Component for PageData {
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

        let main_rect = rect;

        let current_page = self.posts_offset / self.currently_displaying as usize;
        if current_page > 3 {
            self.posts.drain(0..2 * self.currently_displaying as usize);
            self.posts_offset -= self.currently_displaying as usize * 2;
        }

        let mut rects = self.rects_for_posts(rect);
        let size_occupied = rects
            .iter()
            .map(|rect| rect.height)
            .fold(0, |acc, height| acc + height);

        let space_for_padding_available =
            main_rect.height - size_occupied > self.currently_displaying as u16;

        self.render_posts_in_layout(f, &mut rects, space_for_padding_available);
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
