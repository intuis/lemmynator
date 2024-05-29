use std::io::Cursor;
use std::sync::{Arc, Mutex};

use image::GenericImageView;
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
    body: String,
    pub is_focused: bool,
    image_data: Arc<Mutex<Option<ImageData>>>,
    embed_url: Option<url::Url>,
    author: String,
    community: String,
    counts: LemmynatorCounts,
    is_featured_local: bool,
    is_featured_community: bool,
    pub is_body_empty: bool,
}

struct LemmynatorCounts {
    upvotes: i64,
    downvotes: i64,
    comments: i64,
}

struct ImageData {
    image: Box<dyn StatefulProtocol>,
    dimensions: (u32, u32),
}

impl LemmynatorPost {
    pub fn from_lemmy_post(lemmy_post: PostView, ctx: Arc<Ctx>) -> Self {
        let image = Arc::new(Mutex::new(None));

        if let Some(ref url) = lemmy_post.post.thumbnail_url {
            tokio::task::spawn(Self::fetch_image(
                url.as_str().to_string(),
                Arc::clone(&image),
                Arc::clone(&ctx),
            ));
        }

        let embed_url = lemmy_post
            .post
            .url
            .as_ref()
            .map(|db_url| url::Url::parse(db_url.as_str()))
            .transpose()
            .unwrap();

        let body = Self::extract_body(&lemmy_post);

        let counts = LemmynatorCounts {
            upvotes: lemmy_post.counts.upvotes,
            downvotes: lemmy_post.counts.downvotes,
            comments: lemmy_post.counts.downvotes,
        };

        LemmynatorPost {
            name: lemmy_post.post.name,
            is_body_empty: body.is_empty(),
            body,
            community: lemmy_post.community.name,
            author: lemmy_post.creator.name,
            embed_url,
            is_focused: false,
            image_data: image,
            counts,
            is_featured_local: lemmy_post.post.featured_local,
            is_featured_community: lemmy_post.post.featured_community,
        }
    }

    fn extract_body(lemmy_post: &PostView) -> String {
        let unprocessed_body = {
            if let Some(post_body) = &lemmy_post.post.body {
                post_body
            } else if let Some(embed_desc) = &lemmy_post.post.embed_description {
                embed_desc
            } else {
                ""
            }
        };

        unprocessed_body
            .split_inclusive('\n')
            .filter(|line| !line.trim_end().is_empty())
            .collect()
    }

    pub fn is_image_only(&self) -> bool {
        self.body.is_empty() && self.image_data.lock().unwrap().is_some()
    }

    async fn fetch_image(url: String, image: Arc<Mutex<Option<ImageData>>>, ctx: Arc<Ctx>) {
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
                dyn_image.dimensions();
                Some(ImageData {
                    dimensions: dyn_image.dimensions(),
                    image: ctx.picker.lock().unwrap().new_resize_protocol(dyn_image),
                })
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
}

impl Component for LemmynatorPost {
    fn handle_actions(&mut self, _action: Action) -> Option<Action> {
        None
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let inner_rect = rect.inner(&Margin::new(1, 1));

        let post_block = self.post_block();
        f.render_widget(post_block, rect);

        if !self.is_image_only() {
            let [_, image_rect, _, mut text_rect] = Layout::horizontal([
                Constraint::Length(1),
                Constraint::Length(20),
                Constraint::Length(1),
                Constraint::Percentage(75),
            ])
            .areas(inner_rect);

            if let Some(image) = &mut *self.image_data.lock().unwrap() {
                let image_widget = StatefulImage::new(None);
                f.render_stateful_widget(image_widget, image_rect, &mut image.image);
            } else {
                text_rect = inner_rect;
            }

            let mut md_header_encountered = false;
            let body: Vec<_> = self
                .body
                .lines()
                .flat_map(|line| {
                    if line.starts_with('#') {
                        md_header_encountered = true;
                        let trimmed_line = line[0..].trim_start_matches('#').trim_start();
                        vec![Line::styled(trimmed_line, Style::new().bold())]
                    } else if md_header_encountered {
                        let max_width = text_rect.width - 2;
                        Self::wrap_line(line, max_width)
                    } else {
                        vec![Line::from(Self::parse_markdown_url(line))]
                        // vec![Line::raw(line)]
                    }
                })
                .collect();
            let body_paragraph = Paragraph::new(body).wrap(Wrap { trim: false });
            f.render_widget(body_paragraph, text_rect);
        } else {
            let left_padding_percentage = {
                let (width, height) = self.image_data.lock().unwrap().as_ref().unwrap().dimensions;
                if width > height {
                    40
                } else {
                    45
                }
            };
            let [_, image_rect, _] = Layout::horizontal([
                Constraint::Percentage(left_padding_percentage),
                Constraint::Percentage(45),
                Constraint::Percentage(10),
            ])
            .areas(inner_rect);
            if let Some(image) = &mut *self.image_data.lock().unwrap() {
                let image_state = StatefulImage::new(None);
                f.render_stateful_widget(image_state, image_rect, &mut image.image);
            }
        }
    }
}

// UI render related things
impl LemmynatorPost {
    fn post_block(&self) -> Block {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(self.border_style())
            .title(Title::from(self.header()).alignment(Alignment::Left))
            .title(
                Title::from(self.footer_right())
                    .alignment(Alignment::Right)
                    .position(Position::Bottom),
            )
    }

    fn border_style(&self) -> Style {
        if self.is_focused {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default()
        }
    }

    fn footer_right(&self) -> Line {
        let spans = vec![
            Span::styled(
                format!(" c/{}   u/{}  ", self.community, self.author),
                Style::new().white(),
            ),
            Span::styled(format!("  {} ", self.counts.upvotes), Style::new().green()),
            Span::styled(" ", Style::new().white()),
            Span::styled(format!("  {} ", self.counts.downvotes), Style::new().red()),
            Span::styled(" ", Style::new().white()),
            Span::styled(
                format!(" 󰆉 {} ", self.counts.comments),
                Style::new().white(),
            ),
        ];

        let line = if self.is_focused {
            let spans: Vec<_> = spans
                .into_iter()
                .map(|span| span.patch_style(Style::new().bold()))
                .collect();
            Line::default().spans(spans)
        } else {
            Line::default().spans(spans)
        };

        line
    }

    fn header(&self) -> Line {
        let mut spans = vec![];

        if self.is_image_only() {
            spans.push(Span::styled(" ", Style::new().white()));
        }

        if self.is_featured_local {
            spans.push(Span::styled(" 󰐃", Style::new().yellow()))
        }

        if self.is_featured_community {
            spans.push(Span::styled(" 󰐃", Style::new().green()))
        }

        if self.name.len() > 45 {
            let last_char_indice = self.name.char_indices().map(|(i, _)| i).nth(45).unwrap();

            spans.push(Span::styled(
                format!(" {}... ", &self.name[..last_char_indice].trim_end()),
                Style::new().white(),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", self.name),
                Style::new().white(),
            ));
        }

        if let Some(url) = &self.embed_url {
            if let Some(host) = url.host_str() {
                spans.push(Span::styled(format!(" {} ", host), Style::new().white()))
            }
        }

        let spans = if self.is_focused {
            spans
                .into_iter()
                .map(|span| span.patch_style(Style::new().bold()))
                .collect()
        } else {
            spans
        };

        Line::default().spans(spans)
    }

    fn wrap_line(line: &str, max_width: u16) -> Vec<Line> {
        let mut wrapped_lines = Vec::new();
        let mut current_line = String::with_capacity(max_width as usize + 2);
        let mut count = 0;

        for ch in line.chars() {
            if count == 0 {
                current_line.push_str("  ");
                count += 2;
            }

            if count < max_width {
                current_line.push(ch);
                count += 1;
            } else {
                wrapped_lines.push(Line::from(current_line.clone()));
                current_line.clear();
                count = 0;
            }
        }

        if !current_line.is_empty() {
            wrapped_lines.push(Line::from(current_line));
        }
        wrapped_lines
    }

    fn parse_markdown_url(text: &str) -> Vec<Span<'_>> {
        let link_regex = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        let mut parsed_spans = Vec::new();
        let mut last_index = 0;

        for capture in link_regex.captures_iter(text) {
            let full_match = capture.get(0).unwrap();
            let link_text = capture.get(1).unwrap().as_str();
            // let url = capture.get(2).unwrap().as_str();

            parsed_spans.push(Span::styled(
                &text[last_index..full_match.start()],
                Style::new().white(),
            ));
            parsed_spans.push(Span::styled(
                link_text,
                Style::new().fg(Color::Cyan).underlined(),
            ));
            last_index = full_match.end();
        }

        parsed_spans.push(Span::styled(&text[last_index..], Style::new().white()));
        parsed_spans
    }
}
