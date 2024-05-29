use std::sync::Arc;

use crate::{action::Action, app::Ctx};

use super::Component;
use ratatui::{layout::Flex, prelude::*, widgets::Tabs};

#[derive(Clone, Copy)]
pub enum CurrentTab {
    Subscribed = 0,
    Local,
    All,
}

pub struct TabComponent {
    tabs_list: [&'static str; 3],
    pub current_tab: CurrentTab,
    ctx: Arc<Ctx>,
}

impl TabComponent {
    pub fn new(ctx: Arc<Ctx>) -> Self {
        Self {
            tabs_list: ["Subscribed", "Local", "All"],
            current_tab: CurrentTab::Local,
            ctx,
        }
    }
}

impl Component for TabComponent {
    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let center_rect = Layout::horizontal([Constraint::Length(25)])
            .flex(Flex::Center)
            .split(rect)[0];

        let tabs = Tabs::new(self.tabs_list)
            .style(Style::default().white())
            .highlight_style(Style::default().fg(self.ctx.config.general.accent_color.as_ratatui()))
            .select(self.current_tab as usize)
            .divider(symbols::DOT);

        f.render_widget(tabs, center_rect);
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
        None
    }
}
