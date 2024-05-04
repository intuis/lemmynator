pub mod tabs;

use ratatui::prelude::*;

use crate::action::Action;

pub trait Component {
    fn handle_actions(&mut self, _action: Action) -> Option<Action> {
        None
    }

    fn render(&mut self, _f: &mut Frame, _rect: Rect) {}
}
