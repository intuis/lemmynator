use std::{fmt::Display, sync::Arc};

use crate::{action::Action, app::Ctx};

use super::Component;
use intui_tabs::{Tabs, TabsState};
use lemmy_api_common::lemmy_db_schema::{ListingType, SortType};
use ratatui::{layout::Flex, prelude::*, widgets::Paragraph};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CurrentTab {
    Subscribed = 0,
    Local,
    All,
}

impl From<ListingType> for CurrentTab {
    fn from(value: ListingType) -> Self {
        match value {
            ListingType::All => Self::All,
            ListingType::Local => Self::Local,
            ListingType::Subscribed => Self::Subscribed,
            ListingType::ModeratorView => unreachable!(),
        }
    }
}

impl CurrentTab {
    pub fn as_listing_type(&self) -> ListingType {
        match self {
            CurrentTab::Subscribed => ListingType::Subscribed,
            CurrentTab::Local => ListingType::Local,
            CurrentTab::All => ListingType::All,
        }
    }
}

impl Default for CurrentTab {
    fn default() -> Self {
        CurrentTab::Local
    }
}

impl Display for CurrentTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurrentTab::Subscribed => write!(f, "Subscribed"),
            CurrentTab::Local => write!(f, "Local"),
            CurrentTab::All => write!(f, "All"),
        }
    }
}

pub struct TabComponent {
    tabs_sort: [&'static str; 5],
    pub tabs_state: TabsState<CurrentTab>,
    pub current_sort: SortType,
    ctx: Arc<Ctx>,
}

impl TabComponent {
    pub fn new(ctx: Arc<Ctx>) -> Self {
        Self {
            tabs_sort: [
                "!. Hot",
                "@. Active",
                "#. Scaled",
                "$. Controversial",
                "%. New",
            ],
            current_sort: SortType::Hot,
            ctx,
            tabs_state: TabsState::new(vec![
                CurrentTab::Subscribed,
                CurrentTab::Local,
                CurrentTab::All,
            ]),
        }
    }

    pub fn current_sort(&self) -> SortType {
        self.current_sort
    }

    pub fn current_listing_type(&self) -> ListingType {
        self.tabs_state.current().as_listing_type()
    }

    pub fn change_sort(&mut self) {
        match self.current_sort {
            SortType::Hot => self.current_sort = SortType::Active,
            SortType::Active => self.current_sort = SortType::Scaled,
            SortType::Scaled => self.current_sort = SortType::Controversial,
            SortType::New => self.current_sort = SortType::Hot,
            _ => unreachable!(),
        }
    }
}

fn sort_type_index(sort_type: SortType) -> usize {
    match sort_type {
        SortType::Hot => 0,
        SortType::Active => 1,
        SortType::Scaled => 2,
        SortType::Controversial => 3,
        SortType::New => 4,
        _ => unreachable!(),
    }
}

impl Component for TabComponent {
    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let [top_bar] = Layout::vertical([Constraint::Length(1)]).areas(rect);

        let sort_string = format!(" {}  ", self.current_sort().to_string());

        let [listing_type_rect, separator_rect, sort_type_rect] = Layout::horizontal([
            Constraint::Length(34),
            Constraint::Length(3),
            Constraint::Length((sort_string.len()) as u16 + 2),
        ])
        .flex(Flex::Center)
        .areas(top_bar);

        // let listing_type_tabs = Tabs::new(self.tabs_listing_type)
        //     .style(Style::default().white())
        //     .highlight_style(Style::default().fg(self.ctx.config.general.accent_color.as_ratatui()))
        //     .select(self.current_tab as usize)
        //     .divider(symbols::DOT);
        let listing_type_tabs = Tabs::new().color(self.ctx.config.general.accent_color);
        // .style(Style::default().white())
        // .highlight_style(Style::default().fg(self.ctx.config.general.accent_color.as_ratatui()))
        // .select(self.current_tab as usize)
        // .divider(symbols::DOT);

        let separator_paragraph = Paragraph::new(" ⎥ ").bold();

        let spans = vec![Line::from(vec![
            Span::from(" "),
            Span::styled(
                "4",
                Style::default()
                    .underlined()
                    .underline_color(self.ctx.config.general.accent_color),
            ),
            Span::from("."),
            Span::from(sort_string),
            Span::from(""),
        ])];

        let current_sort_paragraph = Paragraph::new(spans).bg(Color::DarkGray);

        f.render_stateful_widget(listing_type_tabs, listing_type_rect, &mut self.tabs_state);
        f.render_widget(separator_paragraph, separator_rect);
        f.render_widget(current_sort_paragraph, sort_type_rect);
    }

    fn handle_actions(&mut self, action: Action) {
        if let Action::ChangeTab(tab) = action {
            match tab {
                1 => self.tabs_state.set(1),
                2 => self.tabs_state.set(2),
                3 => self.tabs_state.set(3),
                _ => (),
            };
            self.ctx.send_action(Action::Render);
        }
    }
}
