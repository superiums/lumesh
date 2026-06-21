use crossterm::event::{KeyCode, KeyEvent as CTermKeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyEvent {
    Paste(String),
    Char(char),
    Ctrl(char),
    Alt(char),
    Shift(char),
    CtrlAlt(char),
    CtrlShift(char),
    AltShift(char),
    CtrlAltShift(char),
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
        let ctrl = ev.modifiers.contains(KeyModifiers::CONTROL);
        let alt = ev.modifiers.contains(KeyModifiers::ALT);
        let shift = ev.modifiers.contains(KeyModifiers::SHIFT);
        match ev.code {
            KeyCode::Char(c) => {
                if ctrl && alt && shift {
                    return KeyEvent::CtrlAltShift(c);
                } else if ctrl && alt {
                    return KeyEvent::CtrlAlt(c);
                } else if ctrl && shift {
                    return KeyEvent::CtrlShift(c);
                } else if alt && shift {
                    return KeyEvent::AltShift(c);
                } else if ctrl && !alt && !shift {
                    if c.to_ascii_uppercase().is_ascii_alphabetic() || c == ' ' {
                        return KeyEvent::Ctrl(c);
                    }
                } else if alt && !ctrl {
                    return KeyEvent::Alt(c);
                } else if shift && !ctrl && !alt {
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
            _ => KeyEvent::Escape,
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
    DeleteToLineStart,
    DeleteToLineEnd,
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
