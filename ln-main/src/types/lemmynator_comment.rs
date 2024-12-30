use std::{
    cmp::max,
    sync::{Arc, Mutex},
};

use image::DynamicImage;
use lemmy_api_common::lemmy_db_views::structs::CommentView;
use ratatui::{
    layout::{Margin, Offset},
    widgets::{Block, Paragraph, Wrap},
};
use ratatui_image::{
    protocol::{ImageSource, StatefulProtocol},
    StatefulImage,
};

use crate::{app, ui::components::Component};

#[derive(Clone)]
pub struct LemmynatorPostComments {
    pub comments: Vec<LemmynatorComment>,
}

#[derive(Clone)]
pub struct LemmynatorComment {
    pub content: String,
    pub author: Author,
    pub replies: Vec<LemmynatorComment>,
}

#[derive(Clone)]
pub struct Author {
    pub name: String,
    pub avatar: AuthorAvatar,
}

#[derive(Clone)]
pub struct AuthorAvatar {
    pub avatar_url: Option<String>,
    pub image: Arc<Mutex<Option<CommentImage>>>,
}

pub enum CommentImage {
    StatelessImage(DynamicImage),
    StatefulImage(((u32, u32), StatefulProtocol)),
}

static DEFAULT_USER_IMAGE: &[u8; 23864] = include_bytes!("../../imgs/user.png");

impl From<Vec<CommentView>> for LemmynatorPostComments {
    fn from(value: Vec<CommentView>) -> Self {
        let mut comments = vec![];
        let mut replies_to_a_comment = vec![];

        for comment_view in value {
            if comment_view.comment.path.split('.').count() != 2 {
                replies_to_a_comment.push(comment_view);
                continue;
            }

            let avatar = if let Some(avatar_url) = comment_view.creator.avatar {
                let image = Arc::new(Mutex::new(None));
                let avatar_url = avatar_url.to_string();

                let avatar_url_clone = avatar_url.to_string();
                let image_clone = image.clone();
                tokio::task::spawn(async move {
                    let bytes = reqwest::get(avatar_url_clone)
                        .await
                        .unwrap()
                        .bytes()
                        .await
                        .unwrap();

                    *image_clone.lock().unwrap() = Some(CommentImage::StatelessImage(
                        image::load_from_memory(&bytes).unwrap(),
                    ));
                });

                AuthorAvatar {
                    avatar_url: Some(avatar_url.to_string()),
                    image,
                }
            } else {
                AuthorAvatar {
                    avatar_url: None,
                    image: Arc::new(Mutex::new(Some(CommentImage::StatelessImage(
                        image::load_from_memory(DEFAULT_USER_IMAGE).unwrap(),
                    )))),
                }
            };

            let author = Author {
                name: comment_view.creator.name,
                avatar,
            };

            let comment = LemmynatorComment {
                content: comment_view.comment.content,
                author,
                replies: vec![],
            };

            comments.push(comment);
        }

        LemmynatorPostComments { comments }
    }
}

pub struct LemmynatorPostCommentsWidget<'a> {
    left_side_width: u16,
    comments: &'a Vec<LemmynatorComment>,
    ctx: Arc<app::Ctx>,
}

impl<'a> LemmynatorPostCommentsWidget<'a> {
    pub fn new(ctx: Arc<app::Ctx>, comments: &'a Vec<LemmynatorComment>) -> Self {
        Self {
            left_side_width: 0,
            comments,
            ctx,
        }
    }

    pub fn left_sife_width(self, left_side_width: u16) -> Self {
        Self {
            comments: self.comments,
            ctx: self.ctx,
            left_side_width,
        }
    }
}

impl<'a> Component for LemmynatorPostCommentsWidget<'a> {
    fn handle_actions(&mut self, action: crate::action::Action) {
        let _ = action;
    }

    fn handle_update_action(&mut self, action: crate::action::UpdateAction) {
        let _ = action;
    }

    fn render(&mut self, f: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {
        let mut place_used: u16 = 0;

        for comment in self.comments {
            let place_to_be_consumed = {
                let mut count = 2;
                for line in comment.content.lines() {
                    let line_by_rect_width = ((line.len() as f64) / (rect.width - 2) as f64).ceil();
                    if line_by_rect_width > 1f64 {
                        count += line_by_rect_width as usize;
                    } else {
                        count += 1;
                    }
                }
                count
            };

            let block = Block::bordered().title(comment.author.name.as_str());
            let mut block_rect = rect.inner(Margin {
                horizontal: 0,
                vertical: place_used,
            });

            if place_used + place_to_be_consumed as u16 >= rect.height {
                continue;
            } else {
                place_used += place_to_be_consumed as u16;
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
                CommentImage::StatelessImage(image) => Some(CommentImage::StatefulImage((
                    (image.width(), image.height()),
                    self.ctx.picker.lock().unwrap().new_resize_protocol(image),
                ))),
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
                            x: -i32::from(self.left_side_width) + 1,
                            y: 0,
                        })
                        .inner(Margin {
                            horizontal: 1,
                            vertical: 1,
                        });

                    avatar_rect.width = self.left_side_width;

                    if avatar_rect.height >= 3 {
                        avatar_rect.height = 2;
                    }

                    let image_res = image.0;
                    let image = &mut image.1;
                    let image_rect = ImageSource::round_pixel_size_to_cells(
                        image_res.0,
                        image_res.1,
                        self.ctx.picker.lock().unwrap().font_size(),
                    );

                    let new_dims = fit_area_proportionally(
                        image_rect.width,
                        image_rect.height,
                        avatar_rect.width,
                        avatar_rect.height,
                    );

                    let avatar_rect = avatar_rect.offset(Offset {
                        x: (self.left_side_width - new_dims.0) as i32 - 3,
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
