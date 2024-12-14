use std::sync::Arc;

use lemmy_api_common::lemmy_db_views::structs::PaginationCursor;
use ratatui::{prelude::*, widgets::Paragraph};

use crate::{
    action::{Action, UpdateAction},
    app::Ctx,
    ui::components::Component,
};

use super::lemmynator_post::LemmynatorPost;

pub struct Page {
    pub posts: Vec<LemmynatorPost>,
    pub next_page: Option<PaginationCursor>,
    pub posts_offset: usize,
    pub currently_focused: u8,
    pub currently_displaying: u8,
    pub all_posts_count: usize,
    ctx: Arc<Ctx>,
}

impl Page {
    pub fn new(ctx: Arc<Ctx>) -> Self {
        Page {
            posts: vec![],
            next_page: None,
            posts_offset: 0,
            currently_focused: 0,
            currently_displaying: 0,
            ctx,
            all_posts_count: 0,
        }
    }

    fn current_post_mut(&mut self) -> &mut LemmynatorPost {
        &mut self.posts[self.posts_offset + self.currently_focused as usize]
    }

    fn current_post(&mut self) -> &LemmynatorPost {
        &self.posts[self.posts_offset + self.currently_focused as usize]
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

    fn current_page(&self) -> usize {
        ((self.all_posts_count / self.currently_displaying as usize)
            - (self.posts.len() - self.posts_offset) / self.currently_displaying as usize)
            + 1
    }

    pub fn render_bottom_bar(&mut self, f: &mut Frame, rect: Rect) {
        if self.currently_displaying != 0 {
            let current_page_paragraph =
                Paragraph::new(format!("{} / ", self.current_page())).alignment(Alignment::Center);
            f.render_widget(current_page_paragraph, rect);
        }
    }
}

impl Component for Page {
    fn handle_actions(&mut self, action: Action) {
        match action {
            Action::Up => {
                self.scroll_up();
                self.ctx.send_action(Action::Render);
            }
            Action::Down => {
                self.scroll_down();
                self.ctx.send_action(Action::Render);
            }
            Action::Confirm => {
                let post = self.current_post().clone();
                self.ctx.send_update_action(UpdateAction::ViewPost(post));
            }
            _ => self.current_post_mut().handle_actions(action),
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

        let is_space_for_padding_available =
            main_rect.height - size_occupied > self.currently_displaying as u16;

        self.render_posts_in_layout(f, &mut rects, is_space_for_padding_available);
    }
}
