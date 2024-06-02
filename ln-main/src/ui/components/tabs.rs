use std::{collections::HashMap, sync::Arc};

use crate::{action::Action, app::Ctx};

use super::Component;
use lemmy_api_common::lemmy_db_schema::{ListingType, SortType};
use ratatui::{layout::Flex, prelude::*, widgets::Tabs};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CurrentTab {
    Subscribed = 0,
    Local,
    All,
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

pub struct TabComponent {
    tabs_listing_type: [&'static str; 3],
    tabs_sort: [&'static str; 5],
    pub current_tab: CurrentTab,
    pub sort_hash: HashMap<CurrentTab, SortType>,
    ctx: Arc<Ctx>,
}

impl TabComponent {
    pub fn new(ctx: Arc<Ctx>) -> Self {
        Self {
            tabs_listing_type: ["1. Subscribed", "2. Local", "3. All"],
            tabs_sort: [
                "!. Hot",
                "@. Active",
                "#. Scaled",
                "$. Controversial",
                "%. New",
            ],
            current_tab: CurrentTab::Local,
            sort_hash: HashMap::new(),
            ctx,
        }
    }

    pub fn current_sort(&self) -> SortType {
        *self
            .sort_hash
            .get(&self.current_tab)
            .unwrap_or(&SortType::Hot)
    }

    pub fn current_listing_type(&self) -> ListingType {
        self.current_tab.as_listing_type()
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
        let [listing_type_rect, sort_type_rect] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(rect);

        let listing_type_rect = Layout::horizontal([Constraint::Length(34)])
            .flex(Flex::Center)
            .split(listing_type_rect)[0];

        let listing_type_tabs = Tabs::new(self.tabs_listing_type)
            .style(Style::default().white())
            .highlight_style(Style::default().fg(self.ctx.config.general.accent_color.as_ratatui()))
            .select(self.current_tab as usize)
            .divider(symbols::DOT);

        let sort_type_rect = Layout::horizontal([Constraint::Length(59)])
            .flex(Flex::Center)
            .split(sort_type_rect)[0];

        let sort_type_tabs = Tabs::new(self.tabs_sort)
            .style(Style::default().white())
            .highlight_style(Style::default().fg(Color::Yellow))
            .select(sort_type_index(self.current_sort()))
            .divider(symbols::DOT);

        f.render_widget(listing_type_tabs, listing_type_rect);
        f.render_widget(sort_type_tabs, sort_type_rect);
    }

    fn handle_actions(&mut self, action: Action) -> Option<Action> {
        if let Action::ChangeTab(tab) = action {
            match tab {
                1 => self.current_tab = CurrentTab::Subscribed,
                2 => self.current_tab = CurrentTab::Local,
                3 => self.current_tab = CurrentTab::All,
                _ => (),
            };
            return Some(Action::Render);
        }

        if let Action::ChangeSort(sort_type) = action {
            self.sort_hash.insert(self.current_tab, sort_type);
            return Some(Action::Render);
        }
        None
    }
}
