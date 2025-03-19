use std::{
    cmp::max,
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use image::DynamicImage;
use lemmy_api_common::lemmy_db_views::structs::CommentView;
use ratatui::{
    layout::{Margin, Offset},
    prelude::Rect,
    widgets::{Block, Paragraph, Wrap},
};
use ratatui_image::{
    protocol::{ImageSource, StatefulProtocol},
    StatefulImage,
};
use tracing::info;

use crate::{
    app::{self, PICKER},
    ui::components::Component,
};

#[derive(Clone)]
pub struct LemmynatorPostComments {
    pub comments: BTreeMap<i32, LemmynatorComment>,
}

#[derive(Clone)]
pub struct LemmynatorComment {
    pub id: i32,
    pub content: String,
    pub author: Author,
    pub path: String,
    pub replies: BTreeMap<i32, LemmynatorComment>,
}

impl LemmynatorComment {
    fn depth(&self) -> u8 {
        u8::try_from(self.path.split('.').count() - 1).unwrap()
    }

    fn how_many_lines_will_consume(&self, width: u16) -> u8 {
        let mut count = 2;
        for line in self.content.lines() {
            let line_by_rect_width = ((line.len() as f64) / (width - 2) as f64).ceil();
            if line_by_rect_width > 1f64 {
                count += line_by_rect_width as usize;
            } else {
                count += 1;
            }
        }
        u8::try_from(count).unwrap()
    }
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
    StatelessImage(DynamicImage, bool),
    StatefulImage((u32, u32), StatefulProtocol, bool),
}

static DEFAULT_USER_IMAGE: &[u8; 23864] = include_bytes!("../../imgs/user.png");

impl From<CommentView> for LemmynatorComment {
    fn from(value: CommentView) -> Self {
        let avatar = if let Some(avatar_url) = value.creator.avatar {
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

                let image = match image::load_from_memory(&bytes) {
                    Ok(image) => Ok(image),
                    Err(e) => match e {
                        image::ImageError::Unsupported(_) => return,
                        _ => Err(e),
                    },
                }
                .unwrap();
                *image_clone.lock().unwrap() = Some(CommentImage::StatelessImage(image, false));
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
                    true,
                )))),
            }
        };

        let author = Author {
            name: value.creator.name,
            avatar,
        };

        LemmynatorComment {
            content: value.comment.content,
            author,
            replies: BTreeMap::new(),
            id: value.comment.id.0,
            path: value.comment.path,
        }
    }
}

impl From<Vec<CommentView>> for LemmynatorPostComments {
    fn from(value: Vec<CommentView>) -> Self {
        let mut comments = BTreeMap::new();
        let mut replies_to_a_comment = vec![];

        for comment_view in value {
            let comment_depth = comment_view.comment.path.split('.').count() - 1;

            if comment_depth != 1 {
                replies_to_a_comment.push(comment_view.into());
                continue;
            }

            let lemmynator_comment: LemmynatorComment = comment_view.into();

            comments.insert(lemmynator_comment.id, lemmynator_comment);
        }

        recursive_function(&mut comments, replies_to_a_comment, 2);

        LemmynatorPostComments { comments }
    }
}

fn recursive_function(
    comments: &mut BTreeMap<i32, LemmynatorComment>,
    replies_to_a_comment: Vec<LemmynatorComment>,
    depth: u8,
) {
    let mut replies_to_a_comment_left = vec![];
    for comment in replies_to_a_comment {
        if comment.depth() != depth {
            replies_to_a_comment_left.push(comment);
            continue;
        }

        let mut path: Vec<_> = comment
            .path
            .split('.')
            .skip(1)
            .map(|x| x.parse::<i32>().unwrap())
            .collect();
        path.pop();

        let mut comments = &mut *comments;
        loop {
            if path.len() == 1 {
                comments
                    .get_mut(&path[0])
                    .unwrap()
                    .replies
                    .insert(comment.id, comment);
                break;
            }

            comments = &mut comments.get_mut(&path[0]).unwrap().replies;
            path.remove(0);
        }
    }

    if !replies_to_a_comment_left.is_empty() {
        recursive_function(comments, replies_to_a_comment_left, depth + 1);
    }
}

pub struct LemmynatorPostCommentsWidget<'a> {
    left_side_width: u16,
    comments: &'a mut BTreeMap<i32, LemmynatorComment>,
    ctx: Arc<app::Ctx>,
}

impl<'a> LemmynatorPostCommentsWidget<'a> {
    pub fn new(ctx: Arc<app::Ctx>, comments: &'a mut BTreeMap<i32, LemmynatorComment>) -> Self {
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

struct LemmynatorCommentWidget<'a> {
    comment: &'a LemmynatorComment,
    left_side_width: u16,
}

impl<'a> LemmynatorCommentWidget<'a> {
    fn new(comment: &'a LemmynatorComment, left_side_width: u16) -> Self {
        Self {
            comment,
            left_side_width,
        }
    }
}

impl<'a> Component for LemmynatorCommentWidget<'a> {
    fn render(&mut self, f: &mut ratatui::Frame, rect: Rect) {
        let block = Block::bordered().title(self.comment.author.name.as_str());
        f.render_widget(block, rect);
        f.render_widget(
            Paragraph::new(self.comment.content.as_str()).wrap(Wrap { trim: true }),
            rect.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );

        let mut avatar_image_lock = self.comment.author.avatar.image.lock().unwrap();

        let new_image = avatar_image_lock.take().and_then(|image| match image {
            CommentImage::StatelessImage(image, is_default) => Some(CommentImage::StatefulImage(
                (image.width(), image.height()),
                PICKER.read().unwrap().new_resize_protocol(image),
                is_default,
            )),
            CommentImage::StatefulImage(res, image, is_default) => {
                Some(CommentImage::StatefulImage(res, image, is_default))
            }
        });

        *avatar_image_lock = new_image;

        match &mut *avatar_image_lock {
            Some(CommentImage::StatefulImage(res, image, is_default)) => {
                let widget_state = StatefulImage::default();

                let vertical_margin = {
                    if *is_default && rect.height == 3 {
                        0
                    } else {
                        1
                    }
                };

                let mut avatar_rect = rect
                    .offset(Offset {
                        x: -i32::from(self.left_side_width) + 1,
                        y: 0,
                    })
                    .inner(Margin {
                        horizontal: 1,
                        vertical: vertical_margin,
                    });

                avatar_rect.width = self.left_side_width;

                if rect.height == 3 && !*is_default {
                    avatar_rect.height = 1;
                } else {
                    avatar_rect.height = 2;
                }

                let image_rect = ImageSource::round_pixel_size_to_cells(
                    res.0,
                    res.1,
                    PICKER.read().unwrap().font_size(),
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

impl<'a> Component for LemmynatorPostCommentsWidget<'a> {
    fn render(&mut self, f: &mut ratatui::Frame, rect: Rect) {
        let mut lines_left: u16 = rect.height;

        for (_, comment) in self.comments.iter_mut() {
            if lines_left <= 1 {
                break;
            }

            let comment_height = comment.how_many_lines_will_consume(rect.width);

            let mut comment_rect = rect.offset(Offset {
                x: 0,
                y: (rect.height - lines_left) as i32,
            });

            if comment_height as u16 >= lines_left {
                info!(
                    "Skipping comment with height of {} (there are {} lines left).",
                    comment_height, lines_left,
                );
                continue;
            } else {
                info!(
                    "Rendering Comment with height of {}, there are {} lines left",
                    comment_height, lines_left
                );
                lines_left -= comment_height as u16;
            }
            comment_rect.height = comment_height as u16;

            info!("This is the comment rect: {:?}", comment_rect);

            LemmynatorCommentWidget::new(&comment, self.left_side_width).render(f, comment_rect);

            if !comment.replies.is_empty() {
                let comment_reply = comment.replies.iter().nth(0).unwrap().1;
                let comment_reply_height =
                    comment_reply.how_many_lines_will_consume(rect.width - 2);

                if comment_reply_height as u16 >= lines_left {
                    continue;
                } else {
                    info!(
                        "Reply by {} will take {} lines of space.",
                        comment_reply.author.name, comment_reply_height
                    );
                    lines_left -= comment_reply_height as u16;
                }

                let mut reply_rect = comment_rect
                    .offset(Offset {
                        x: 2,
                        y: comment_rect.height as i32,
                    })
                    .inner(Margin {
                        horizontal: 2,
                        vertical: 0,
                    });

                reply_rect.height = comment_reply_height as u16;

                info!("This is the reply rect: {:?}", reply_rect);

                LemmynatorCommentWidget::new(&comment_reply, self.left_side_width)
                    .render(f, reply_rect);
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
