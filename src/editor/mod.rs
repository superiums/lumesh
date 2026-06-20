mod buffer;
pub mod editor;
mod history;
mod key;
mod kring;

pub use editor::{Editor, Completer, Highlighter, Hinter, ReadlineError, CompletionItem, EditorTheme, Validator, ValidationResult};
pub use history::History;
pub use key::{Cmd, KeyEvent};
pub use buffer::LineBuffer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_basics() {
        let mut buf = LineBuffer::new();
        assert!(buf.is_empty());
        buf.insert('h');
        buf.insert('i');
        assert_eq!(buf.text(), "hi");
        assert_eq!(buf.cursor(), 2);
        buf.move_left();
        assert_eq!(buf.cursor(), 1);
        buf.delete_word_before();
        assert_eq!(buf.text(), "i");
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn test_history() {
        let mut hist = History::new();
        hist.add("echo hello".into());
        hist.add("ls -la".into());
        assert_eq!(hist.len(), 2);

        assert_eq!(hist.previous(""), Some("ls -la"));
        assert_eq!(hist.previous(""), Some("echo hello"));

        hist.add("new entry".into());
        assert_eq!(hist.len(), 3);
    }

    #[test]
    fn test_kill_ring() {
        let mut kr = kring::KillRing::new();
        kr.push("killed text".into());
        assert_eq!(kr.yank(), Some("killed text"));
    }

    #[test]
    fn test_buffer_insert_str() {
        let mut buf = LineBuffer::new();
        buf.insert_str("hello world");
        assert_eq!(buf.text(), "hello world");
        assert_eq!(buf.cursor(), 11);
        buf.move_to_start();
        buf.move_word_right();
        assert_eq!(buf.cursor(), 5);
    }
}
