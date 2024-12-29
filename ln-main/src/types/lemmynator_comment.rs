use std::sync::{Arc, Mutex};

use image::DynamicImage;
use lemmy_api_common::lemmy_db_views::structs::CommentView;
use ratatui_image::protocol::StatefulProtocol;

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
