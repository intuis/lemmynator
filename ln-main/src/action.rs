use crossterm::event::{KeyCode, KeyEvent};
use lemmy_api_common::{
    comment::GetCommentsResponse,
    lemmy_db_schema::{ListingType, SortType},
    person::GetUnreadCountResponse,
    post::GetPostsResponse,
};

use crate::{tui::Event, types::LemmynatorPost};

#[derive(Clone)]
pub enum UpdateAction {
    NewPage(ListingType, SortType, GetPostsResponse),
    ViewPost(LemmynatorPost),
    CommentsForCurrentPost(GetCommentsResponse),
    UpdateUnreadsCount(GetUnreadCountResponse),
}

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    ForceQuit,
    Render,
    Up,
    Down,
    VoteUp,
    VoteDown,
    Confirm,
    ShowHelp,
    SwitchToInputMode,
    SwitchToNormalMode,
    ChangeFocus,
    ChangeSort,
    ChangeTab(u8),
    ChangeSubTab(u8),
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
        KeyCode::Char('J') => Some(A::VoteDown),
        KeyCode::Char('K') => Some(A::VoteUp),
        KeyCode::Char('q') => Some(A::Quit),
        KeyCode::Char('?') => Some(A::ShowHelp),
        KeyCode::Char(n @ '1'..='3') => {
            Some(A::ChangeTab(n.to_digit(10).expect("This is ok") as u8))
        }
        KeyCode::Char('!') => Some(A::ChangeSubTab(1)),
        KeyCode::Char('@') => Some(A::ChangeSubTab(2)),
        KeyCode::Char('#') => Some(A::ChangeSubTab(3)),
        KeyCode::Char('4') => Some(A::ChangeSort),
        KeyCode::Enter => Some(Action::Confirm),
        _ => None,
    }
}
