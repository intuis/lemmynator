use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::StatefulImage;

use crate::action::{Action, UpdateAction};

use super::{components::Component, listing::lemmynator_post::LemmynatorPost};

pub struct PostView {
    pub post: LemmynatorPost,
    zoom_amount: u16,
}

impl PostView {
    pub fn new(post: LemmynatorPost) -> Self {
        Self {
            post,
            zoom_amount: 0,
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
        let [top_bar, rect] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(rect);

        let spans = vec![
            Span::raw(" << Press "),
            Span::styled("q", Style::default().magenta().underlined()),
            Span::raw(" to go back."),
        ];

        let how_to_quit = Paragraph::new(Line::from(spans));

        f.render_widget(how_to_quit, top_bar);

        let [_, rect, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Percentage(75),
            Constraint::Fill(1),
        ])
        .areas(rect);

        if let Some(image) = &mut *self.post.image_data.lock().unwrap() {
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

            let [_, image_rect, _, post_rect, _, comments_rect] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Percentage(50 + self.zoom_amount),
                Constraint::Length(1),
                Constraint::Length(desc_lines),
                Constraint::Length(1),
                Constraint::Percentage(50 - self.zoom_amount),
            ])
            .areas(rect);

            let image_state = StatefulImage::new(None);

            f.render_stateful_widget(image_state, image_rect, &mut image.image);
            let body_paragraph = self.post.desc_md_paragraph(post_rect);
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
            f.render_widget(body_paragraph, post_rect);

            let comments_rect = comments_rect.inner(Margin {
                horizontal: 0,
                vertical: 1,
            });

            if let Some(comments) = &self.post.comments {
                let mut place_used: u16 = 0;
                for (idx, comment) in comments.iter().enumerate() {
                    let place_to_be_consumed = {
                        let mut count = 0;
                        for line in comment.comment.content.lines() {
                            if (line.len() / (comments_rect.width - 2) as usize) > 1 {
                                count += line.len() / (comments_rect.width - 2) as usize;
                            } else {
                                count += 1;
                            }
                        }
                        count += 2;
                        count
                    };

                    let block = Block::bordered().title(comment.creator.name.as_str());
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
                        Paragraph::new(comment.comment.content.as_str()),
                        block_rect.inner(Margin {
                            horizontal: 1,
                            vertical: 1,
                        }),
                    );
                }
            }
        }
    }
}
