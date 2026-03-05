use crossterm::event::{KeyCode, KeyModifiers};

pub struct AppKeyEvent(crossterm::event::KeyEvent);

impl From<crossterm::event::KeyEvent> for AppKeyEvent {
    fn from(value: crossterm::event::KeyEvent) -> Self {
        Self(value)
    }
}

impl AppKeyEvent {
    pub fn is_ctrl_c(&self) -> bool {
        if self.0.modifiers == KeyModifiers::CONTROL
            && (self.0.code == KeyCode::Char('c') || self.0.code == KeyCode::Char('C'))
        {
            return true;
        }
        false
    }

    fn keybinding(&self) -> (KeyCode, KeyModifiers) {
        match self.0.code {
            KeyCode::Char(e) => {
                let modifier = if e.is_uppercase() {
                    KeyModifiers::NONE
                } else {
                    self.0.modifiers
                };
                (self.0.code, modifier)
            }
            _ => (self.0.code, self.0.modifiers),
        }
    }
}
