use std::{cmp::max, fmt::Display};

use intui_tabs::{Tabs, TabsState};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Offset, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{picker, protocol::ImageSource, StatefulImage};

use crate::{
    action::{Action, UpdateAction},
    types::{CommentImage, LemmynatorPost},
};

use super::components::Component;

#[derive(Clone, Copy)]
enum CurrentTab {
    Overview,
    Post,
    Comments,
}

impl Default for CurrentTab {
    fn default() -> Self {
        Self::Overview
    }
}

impl Display for CurrentTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurrentTab::Overview => write!(f, "Overview"),
            CurrentTab::Post => write!(f, "Post"),
            CurrentTab::Comments => write!(f, "Comments"),
        }
    }
}

pub struct PostView {
    pub post: LemmynatorPost,
    tabs_state: TabsState<CurrentTab>,
    zoom_amount: u16,
}

impl PostView {
    pub fn new(post: LemmynatorPost) -> Self {
        Self {
            post,
            zoom_amount: 0,
            tabs_state: TabsState::new(vec![
                CurrentTab::Overview,
                CurrentTab::Post,
                CurrentTab::Comments,
            ]),
        }
    }
}

impl Component for PostView {
    fn handle_actions(&mut self, action: Action) {
        match action {
            Action::Up => {
                self.zoom_amount = self.zoom_amount.saturating_sub(5);
                self.post.ctx.send_action(Action::Render);
            }
            Action::Down => {
                self.zoom_amount += 5;
                self.post.ctx.send_action(Action::Render);
            }
            _ => (),
        }
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        let _ = action;
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [sub_tab, _, rect, keybinds_bar] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(rect);

        let tabs = Tabs::new()
            .center(true)
            .beginner_mode(true)
            .sub_tab(true)
            .color(self.post.ctx.config.general.accent_color);
        f.render_stateful_widget(tabs, sub_tab, &mut self.tabs_state);

        let spans = vec![
            Span::raw(" << Press "),
            Span::styled(
                "q",
                Style::default()
                    .underlined()
                    .fg(self.post.ctx.config.general.accent_color),
            ),
            Span::raw(" to go back."),
        ];

        let how_to_quit = Paragraph::new(Line::from(spans));

        f.render_widget(how_to_quit, keybinds_bar);

        let [left_side_rect, rect, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Percentage(75),
            Constraint::Fill(1),
        ])
        .areas(rect);

        let mut body_rect: Option<Rect> = None;
        let mut comments_rect: Option<Rect> = None;

        let desc_lines = {
            let mut count = 0;
            for line in self.post.body.lines() {
                if (u16::try_from(line.len()).unwrap() / rect.width) >= 1 {
                    count += (u16::try_from(line.len()).unwrap() / rect.width) + 1;
                } else {
                    count += 1;
                }
            }
            count
        };

        if let Some(image) = &mut *self.post.image_data.lock().unwrap() {
            let [_, image_rect, _, image_body_rect, _, image_comments_rect] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Percentage(50 + self.zoom_amount),
                Constraint::Length(1),
                Constraint::Length(desc_lines),
                Constraint::Length(1),
                Constraint::Percentage(50 - self.zoom_amount),
            ])
            .areas(rect);

            body_rect = Some(image_body_rect);
            comments_rect = Some(image_comments_rect);

            let image_state = StatefulImage::default();

            f.render_stateful_widget(image_state, image_rect, &mut image.image);
            f.render_widget(
                Block::new().borders(Borders::TOP).title_top(format!(
                    "{} {} ",
                    self.post.border_separator(),
                    self.post.name
                )),
                rect.inner(Margin {
                    horizontal: 0,
                    vertical: 1,
                }),
            );
        } else {
            let [_, post_body_rect, post_comments_rect] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(desc_lines + 1),
                Constraint::Fill(1),
            ])
            .areas(rect);

            body_rect = Some(post_body_rect);
            comments_rect = Some(post_comments_rect);
        }

        if let Some(body_rect) = body_rect {
            let body_paragraph = self.post.desc_md_paragraph(body_rect);
            f.render_widget(body_paragraph, body_rect);
        }

        if let Some(comments_rect) = comments_rect {
            f.render_widget(
                Block::new()
                    .borders(Borders::TOP)
                    .title_top(self.post.footer())
                    .title_alignment(Alignment::Right)
                    .title(
                        Title::from(format!("{} Comments ", self.post.border_separator()))
                            .alignment(Alignment::Left),
                    ),
                comments_rect,
            );

            let comments_rect = comments_rect.inner(Margin {
                horizontal: 0,
                vertical: 1,
            });

            if let Some(comments) = &self.post.comments {
                if comments.comments.is_empty() {
                    let no_comments_paragraph =
                        Paragraph::new("\nNo comments yet! Be the first to share your thoughts.")
                            .dim()
                            .centered();
                    f.render_widget(no_comments_paragraph, comments_rect);
                } else {
                    let mut place_used: u16 = 0;
                    for comment in &comments.comments {
                        let place_to_be_consumed = {
                            let mut count = 0;
                            for line in comment.content.lines() {
                                let line_by_rect_width =
                                    (line.len() as f64 / (comments_rect.width - 2) as f64).ceil();
                                if line_by_rect_width > 1f64 {
                                    count += line_by_rect_width as usize;
                                } else {
                                    count += 1;
                                }
                            }
                            count += 2;
                            count
                        };

                        let block = Block::bordered().title(comment.author.name.as_str());
                        let mut block_rect = comments_rect.inner(Margin {
                            horizontal: 0,
                            vertical: place_used,
                        });

                        place_used += place_to_be_consumed as u16;

                        if place_used >= comments_rect.height {
                            break;
                        }

                        block_rect.height = place_to_be_consumed as u16;
                        f.render_widget(block, block_rect);
                        f.render_widget(
                            Paragraph::new(comment.content.as_str()).wrap(Wrap { trim: true }),
                            block_rect.inner(Margin {
                                horizontal: 1,
                                vertical: 1,
                            }),
                        );

                        let mut avatar_image_lock = comment.author.avatar.image.lock().unwrap();

                        let new_image = avatar_image_lock.take().and_then(|image| match image {
                            CommentImage::StatelessImage(image) => {
                                Some(CommentImage::StatefulImage((
                                    (image.width(), image.height()),
                                    self.post
                                        .ctx
                                        .picker
                                        .lock()
                                        .unwrap()
                                        .new_resize_protocol(image),
                                )))
                            }
                            CommentImage::StatefulImage(stateful) => {
                                Some(CommentImage::StatefulImage(stateful))
                            }
                        });

                        *avatar_image_lock = new_image;

                        match &mut *avatar_image_lock {
                            Some(CommentImage::StatefulImage(ref mut image)) => {
                                let widget_state = StatefulImage::default();
                                let mut avatar_rect = block_rect
                                    .offset(Offset {
                                        x: -i32::from(left_side_rect.width) + 1,
                                        y: 0,
                                    })
                                    .inner(Margin {
                                        horizontal: 1,
                                        vertical: 1,
                                    });

                                avatar_rect.width = left_side_rect.width;

                                let image_res = image.0;
                                let image = &mut image.1;
                                let image_rect = ImageSource::round_pixel_size_to_cells(
                                    image_res.0,
                                    image_res.1,
                                    self.post.ctx.picker.lock().unwrap().font_size(),
                                );

                                let new_dims = fit_area_proportionally(
                                    image_rect.width,
                                    image_rect.height,
                                    avatar_rect.width,
                                    avatar_rect.height,
                                );

                                let avatar_rect = avatar_rect.offset(Offset {
                                    x: (left_side_rect.width - new_dims.0) as i32 - 3,
                                    y: 0,
                                });

                                f.render_stateful_widget(widget_state, avatar_rect, image);
                            }
                            Some(_) => unreachable!(),
                            None => (),
                        }
                    }
                }
            }
        }
    }
}

fn fit_area_proportionally(width: u16, height: u16, nwidth: u16, nheight: u16) -> (u16, u16) {
    let wratio = nwidth as f64 / width as f64;
    let hratio = nheight as f64 / height as f64;

    let ratio = f64::min(wratio, hratio);

    let nw = max((width as f64 * ratio).round() as u64, 1);
    let nh = max((height as f64 * ratio).round() as u64, 1);

    if nw > u64::from(u16::MAX) {
        let ratio = u16::MAX as f64 / width as f64;
        (u16::MAX, max((height as f64 * ratio).round() as u16, 1))
    } else if nh > u64::from(u16::MAX) {
        let ratio = u16::MAX as f64 / height as f64;
        (max((width as f64 * ratio).round() as u16, 1), u16::MAX)
    } else {
        (nw as u16, nh as u16)
    }
}
