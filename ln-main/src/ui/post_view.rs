use std::{fmt::Display, sync::Arc};

use intui_tabs::{Tabs, TabsState};
use ln_config::CONFIG;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Paragraph},
    Frame,
};

use crate::{
    action::{Action, UpdateAction},
    types::{LemmynatorPost, LemmynatorPostCommentsWidget},
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
            Action::ChangeSubTab(n) => {
                self.tabs_state.set(n.into());
            }
            _ => (),
        }
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        let _ = action;
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [sub_tab, _, main_rect, keybinds_bar_rect] = Layout::vertical([
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
            .color(CONFIG.general.accent_color);
        f.render_stateful_widget(tabs, sub_tab, &mut self.tabs_state);

        let spans = vec![
            Span::raw(" << Press "),
            Span::styled(
                "q",
                Style::default()
                    .underlined()
                    .fg(CONFIG.general.accent_color),
            ),
            Span::raw(" to go back."),
        ];

        let how_to_quit = Paragraph::new(Line::from(spans));

        f.render_widget(how_to_quit, keybinds_bar_rect);

        let [left_side_rect, rect, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Percentage(75),
            Constraint::Fill(1),
        ])
        .areas(main_rect);

        let body_rect: Option<Rect>;
        let comments_rect: Option<Rect>;

        match self.tabs_state.current() {
            CurrentTab::Overview => {
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
                    let [_, image_rect, _, image_body_rect, _, image_comments_rect] =
                        Layout::vertical([
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

                    image.render(f, image_rect, Arc::clone(&self.post.ctx));
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
                            .title_top(self.post.footer().right_aligned())
                            .title(Title::from(format!(
                                "{} Comments ",
                                self.post.border_separator()
                            )))
                            .title_alignment(Alignment::Left),
                        comments_rect,
                    );

                    let comments_rect = comments_rect.inner(Margin {
                        horizontal: 0,
                        vertical: 1,
                    });

                    if let Some(comments) = &mut self.post.comments {
                        if comments.comments.is_empty() {
                            let no_comments_paragraph = Paragraph::new(
                                "\nNo comments yet! Be the first to share your thoughts.",
                            )
                            .dim()
                            .centered();
                            f.render_widget(no_comments_paragraph, comments_rect);
                        } else {
                            LemmynatorPostCommentsWidget::new(
                                self.post.ctx.clone(),
                                &mut comments.comments,
                            )
                            .left_sife_width(left_side_rect.width)
                            .render(f, comments_rect);
                        }
                    }
                }
            }
            CurrentTab::Post => todo!(),
            CurrentTab::Comments => {
                let comments_rect = rect;
                if let Some(comments) = &mut self.post.comments {
                    if comments.comments.is_empty() {
                        let no_comments_paragraph = Paragraph::new(
                            "\nNo comments yet! Be the first to share your thoughts.",
                        )
                        .dim()
                        .centered();
                        f.render_widget(no_comments_paragraph, comments_rect);
                    } else {
                        LemmynatorPostCommentsWidget::new(
                            self.post.ctx.clone(),
                            &mut comments.comments,
                        )
                        .left_sife_width(left_side_rect.width)
                        .render(f, comments_rect);
                    }
                }
            }
        }
    }
}
