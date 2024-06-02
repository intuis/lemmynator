use crossterm::event::{KeyCode, KeyEvent};
use lemmy_api_common::lemmy_db_schema::SortType;

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
    ChangeSort(SortType),
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
    use Action as A;

    match key.code {
        KeyCode::Tab => Some(A::ChangeFocus),
        KeyCode::Char('j') | KeyCode::Down => Some(A::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(A::Up),
        KeyCode::Char('q') => Some(A::Quit),
        KeyCode::Char('?') => Some(A::ShowHelp),
        KeyCode::Char(n @ '1'..='3') => {
            Some(A::ChangeTab(n.to_digit(10).expect("This is ok") as u8))
        }
        KeyCode::Char(n) => match n {
            '!' => Some(A::ChangeSort(SortType::Hot)),
            '@' => Some(A::ChangeSort(SortType::Active)),
            '#' => Some(A::ChangeSort(SortType::Scaled)),
            '$' => Some(A::ChangeSort(SortType::Controversial)),
            '%' => Some(A::ChangeSort(SortType::New)),
            _ => None,
        },
        KeyCode::Enter => Some(Action::Confirm),
        _ => None,
    }
}
