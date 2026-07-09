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
    AltEnter,
    CtrlEnter,
    ShiftEnter,
    Enter,
    Tab,
    BackTab,
    Backspace,
    CtrlBackspace,
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
    None,
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
                    // Kitty下不需要
                    // 过滤掉终端已经转换过的控制字符（如 Ctrl+A → \x01）
                    // if c.to_ascii_uppercase().is_ascii_alphabetic() || c == ' ' {
                    return KeyEvent::Ctrl(c);
                    // }
                } else if alt && !ctrl {
                    return KeyEvent::Alt(c);
                } else if shift && !ctrl && !alt {
                    return KeyEvent::Char(shift_char(
                        c,
                        ev.state == crossterm::event::KeyEventState::CAPS_LOCK,
                    ));
                    // return KeyEvent::Shift(c);
                }
                // } else if shift && !ctrl && !alt {
                //     // Kitty 协议下 c 已经是 shift 后的字符（'A'、'!'等）
                //     // 直接作为普通字符处理，无需区分
                //     return KeyEvent::Char(c);
                // }
                if ev.state == crossterm::event::KeyEventState::CAPS_LOCK {
                    KeyEvent::Char(c.to_ascii_uppercase())
                } else {
                    KeyEvent::Char(c)
                }
            }
            // KeyCode::Enter => KeyEvent::Enter,
            KeyCode::Enter => {
                if alt {
                    KeyEvent::AltEnter
                } else if ctrl {
                    KeyEvent::CtrlEnter
                } else if shift {
                    KeyEvent::ShiftEnter
                    // KeyEvent::Ctrl('m') // Ctrl+Enter 在标准终端中等同于 Ctrl+M
                } else {
                    KeyEvent::Enter
                }
            }
            KeyCode::Tab => {
                if shift {
                    KeyEvent::BackTab
                } else {
                    KeyEvent::Tab
                }
            }
            KeyCode::BackTab => KeyEvent::BackTab,
            KeyCode::Backspace => {
                if ctrl {
                    KeyEvent::CtrlBackspace
                } else {
                    KeyEvent::Backspace
                }
            }
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
            KeyCode::Modifier(_) => KeyEvent::None,
            KeyCode::Media(_) => KeyEvent::None,
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
    ToggleSudo(String),
    Noop,
}

fn shift_char(c: char, is_cap: bool) -> char {
    match c {
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '=' => '+',
        '[' => '{',
        ']' => '}',
        '\\' => '|',
        ';' => ':',
        '\'' => '"',
        ',' => '<',
        '.' => '>',
        '/' => '?',
        '`' => '~',
        c if is_cap => c,
        c => c.to_ascii_uppercase(),
    }
}
