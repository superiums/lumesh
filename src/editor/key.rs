use crossterm::event::{KeyCode, KeyModifiers, KeyEvent as CTermKeyEvent};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyEvent {
    Char(char),
    Ctrl(char),
    Alt(char),
    Shift(char),
    Enter,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

impl From<CTermKeyEvent> for KeyEvent {
    fn from(ev: CTermKeyEvent) -> Self {
        match ev.code {
            KeyCode::Char(c) => {
                if ev.modifiers.contains(KeyModifiers::CONTROL) && !ev.modifiers.contains(KeyModifiers::ALT) {
                    if c.to_ascii_uppercase().is_ascii_alphabetic() || c == ' ' {
                        return KeyEvent::Ctrl(c);
                    }
                }
                if ev.modifiers.contains(KeyModifiers::ALT) && !ev.modifiers.contains(KeyModifiers::CONTROL) {
                    return KeyEvent::Alt(c);
                }
                if ev.modifiers.contains(KeyModifiers::SHIFT) && !ev.modifiers.contains(KeyModifiers::CONTROL) && !ev.modifiers.contains(KeyModifiers::ALT) {
                    return KeyEvent::Shift(c);
                }
                KeyEvent::Char(c)
            }
            KeyCode::Enter => KeyEvent::Enter,
            KeyCode::Tab => {
                if ev.modifiers.contains(KeyModifiers::SHIFT) {
                    KeyEvent::BackTab
                } else {
                    KeyEvent::Tab
                }
            }
            KeyCode::Backspace => KeyEvent::Backspace,
            KeyCode::Delete => KeyEvent::Delete,
            KeyCode::Esc => KeyEvent::Escape,
            KeyCode::Left => KeyEvent::Left,
            KeyCode::Right => KeyEvent::Right,
            KeyCode::Up => KeyEvent::Up,
            KeyCode::Down => KeyEvent::Down,
            KeyCode::Home => KeyEvent::Home,
            KeyCode::End => KeyEvent::End,
            KeyCode::PageUp => KeyEvent::PageUp,
            KeyCode::PageDown => KeyEvent::PageDown,
            KeyCode::F(n) => KeyEvent::F(n),
            _ => {
                // Uncommon keys
                if ev.modifiers.contains(KeyModifiers::ALT) && ev.modifiers.contains(KeyModifiers::SHIFT) {
                    if let KeyCode::Char(c) = ev.code {
                        return KeyEvent::Alt(c);
                    }
                }
                if ev.modifiers.contains(KeyModifiers::CONTROL) && ev.modifiers.contains(KeyModifiers::ALT) {
                    if let KeyCode::Char(c) = ev.code {
                        return KeyEvent::Ctrl(c);
                    }
                }
                KeyEvent::Escape
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Insert(char),
    InsertStr(String),
    InsertStrAtBeginning(String),
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveToStart,
    MoveToEnd,
    MoveWordLeft,
    MoveWordRight,
    DeleteWordBefore,
    DeleteWordAfter,
    DeleteToStart,
    DeleteToEnd,
    AcceptLine,
    Cancel,
    Complete,
    HistoryPrevious,
    HistoryNext,
    HistorySearch,
    Yank,
    TransposeChars,
    ClearScreen,
    AcceptHint,
    AcceptHintWord(u8),
    ClearBuffer,
    ToggleSudo,
    Noop,
}
