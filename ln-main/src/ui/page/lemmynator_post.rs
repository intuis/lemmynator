use std::io::Cursor;
use std::sync::{Arc, Mutex};

use lemmy_api_common::lemmy_db_views::structs::PostView;
use ratatui::prelude::*;
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;

use crate::action::Action;
use crate::app::Ctx;
use crate::ui::components::Component;

pub struct LemmynatorPost {
    name: String,
    body: Option<String>,
    pub is_focused: bool,
    decoded_image: Arc<Mutex<Option<Box<dyn StatefulProtocol>>>>,
    embed_description: Option<String>,
    embed_url: Option<url::Url>,
    author: String,
    community: String,
    downvotes: i64,
    comments: i64,
    upvotes: i64,
}

impl LemmynatorPost {
    fn header(&self) -> String {
        if let Some(url) = &self.embed_url {
            if let Some(host) = url.host_str() {
                format!(" {}  {} ", self.name, host)
            } else {
                format!(" {} ", self.name)
            }
        } else {
            format!(" {} ", self.name)
        }
    }

    fn footer_right(&self) -> Line {
        // Line::default().spans(vec![format!(
        //     " c/{}  u/{}  ",
        //     self.community, self.author
        // )]);
        let line = Line::default().spans(vec![
            Span::raw(format!(" c/{}  u/{}  ", self.community, self.author)),
            Span::styled(format!(" {}  ", self.upvotes), Style::new().green()),
            Span::styled(format!(" {}  ", self.downvotes), Style::new().red()),
            Span::raw(format!("󰆉 {} ", self.comments)),
        ]);

        line
        // format!(
        //     " c/{}  u/{}   {}  {}  󰆉 {} ",
        //     self.community,
        //     self.author,
        //     self.upvotes.to_string().green(),
        //     self.downvotes.to_string().red(),
        //     self.comments
        // )
    }

    async fn fetch_image(
        url: String,
        image: Arc<Mutex<Option<Box<dyn StatefulProtocol>>>>,
        ctx: Arc<Ctx>,
    ) {
        let new_image = {
            let res = ctx.client.get(url).send().await.unwrap();
            Some(res.bytes().await.unwrap())
        };

        let new_image = if let Some(image) = new_image {
            let dyn_image_res = image::io::Reader::new(Cursor::new(image))
                .with_guessed_format()
                .unwrap()
                .decode();
            if let Ok(dyn_image) = dyn_image_res {
                Some(ctx.picker.lock().unwrap().new_resize_protocol(dyn_image))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(new_image) = new_image {
            *image.lock().unwrap() = Some(new_image);
        }
        ctx.action_tx.send(Action::Render).unwrap();
    }

    pub fn from_lemmy_post(lemmy_post: PostView, ctx: Arc<Ctx>) -> Self {
        let decoded_image = Arc::new(Mutex::new(None));

        if let Some(url) = lemmy_post.post.thumbnail_url {
            tokio::task::spawn(Self::fetch_image(
                url.as_str().to_string(),
                Arc::clone(&decoded_image),
                Arc::clone(&ctx),
            ));
        }

        let embed_url = lemmy_post
            .post
            .url
            .and_then(|db_url| Some(url::Url::parse(db_url.as_str()).unwrap()));

        LemmynatorPost {
            name: lemmy_post.post.name,
            body: lemmy_post.post.body,
            community: lemmy_post.community.name,
            author: lemmy_post.creator.name,
            embed_description: lemmy_post.post.embed_description,
            embed_url,
            is_focused: false,
            decoded_image,
            upvotes: lemmy_post.counts.upvotes,
            downvotes: lemmy_post.counts.downvotes,
            comments: lemmy_post.counts.comments,
        }
    }
}

impl Component for LemmynatorPost {
    fn handle_actions(&mut self, _action: Action) -> Option<Action> {
        None
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let inner_rect = rect.inner(&Margin::new(1, 1));

        let border_style = {
            if self.is_focused {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            }
        };

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Title::from(self.header()).alignment(Alignment::Left))
            .title(
                Title::from(self.footer_right())
                    .alignment(Alignment::Right)
                    .position(Position::Bottom),
            );

        f.render_widget(block, rect);

        let [_, image_rect, _, mut text_rect] = Layout::horizontal([
            Constraint::Length(1),
            Constraint::Length(20),
            Constraint::Length(1),
            Constraint::Percentage(75),
        ])
        .areas(inner_rect);

        if let Some(image) = &mut *self.decoded_image.lock().unwrap() {
            let image_state = StatefulImage::new(None);
            f.render_stateful_widget(image_state, image_rect, image);
        } else {
            text_rect = inner_rect;
        }

        if let Some(body) = &self.body {
            let text = Paragraph::new(body.as_str()).wrap(Wrap { trim: false });
            f.render_widget(text, text_rect);
        } else if let Some(embed_desc) = &self.embed_description {
            let text = Paragraph::new(embed_desc.as_str()).wrap(Wrap { trim: false });
            f.render_widget(text, text_rect);
        }

        self.is_focused = false;
    }
}
