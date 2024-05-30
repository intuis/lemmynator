use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::Event;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Render,
    Up,
    Down,
    Confirm,
    ShowHelp,
    SwitchToInputMode,
    SwitchToNormalMode,
    ChangeFocus,
    ChangeTab(u8),
    Input(KeyEvent),
}

impl Action {
    pub const fn is_render(&self) -> bool {
        matches!(self, Self::Render)
    }
}

#[derive(Clone, Copy)]
pub enum Mode {
    Input,
    Normal,
}

pub fn event_to_action(mode: Mode, event: Event) -> Option<Action> {
    match event {
        Event::Error => todo!(),
        Event::Render => Some(Action::Render),
        Event::Key(key) if matches!(mode, Mode::Input) => Some(Action::Input(key)),
        Event::Key(key) => keycode_to_action(key),
    }
}

fn keycode_to_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Tab => Some(Action::ChangeFocus),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('?') => Some(Action::ShowHelp),
        KeyCode::Char(n @ '1'..='9') => {
            Some(Action::ChangeTab(n.to_digit(10).expect("This is ok") as u8))
        }
        KeyCode::Enter => Some(Action::Confirm),
        _ => None,
    }
}
