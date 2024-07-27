pub mod tabs;

use ratatui::prelude::*;

use crate::action::{Action, UpdateAction};

pub trait Component {
    fn handle_actions(&mut self, action: Action) {
        let _ = action;
    }

    fn handle_update_action(&mut self, action: UpdateAction) {
        let _ = action;
    }

    fn render(&mut self, _f: &mut Frame, _rect: Rect) {}
}
