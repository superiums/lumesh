// 复用现有的高亮逻辑

use crate::{Diagnostic, TokenKind, tokenize};

pub fn highlight(line: &str) -> String {
    let (tokens, diagnostics) = tokenize(line);
    // dbg!(tokens);

    let mut result = String::new();
    let mut is_colored = false;

    for (token, diagnostic) in tokens.iter().zip(&diagnostics) {
        match (token.kind, token.range.to_str(line)) {
            (TokenKind::ValueSymbol, b) => {
                result.push_str("\x1b[95m");
                is_colored = true;
                result.push_str(b);
            }
            (
                TokenKind::Punctuation,
                o @ ("@" | "\'" | "=" | "|" | ">>" | "<<" | ">!" | "->" | "~>"),
            ) => {
                result.push_str("\x1b[96m");
                is_colored = true;
                result.push_str(o);
            }
            (TokenKind::Punctuation, o) => {
                if is_colored {
                    result.push_str("\x1b[m\x1b[0m");
                    is_colored = false;
                }
                result.push_str(o);
            }
            (TokenKind::Keyword, k) => {
                result.push_str("\x1b[95m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::Operator, k) => {
                result.push_str("\x1b[38;5;220m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorPrefix, k) => {
                result.push_str("\x1b[38;5;221m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorInfix, k) => {
                result.push_str("\x1b[38;5;222m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::OperatorPostfix, k) => {
                result.push_str("\x1b[38;5;223m");
                is_colored = true;
                result.push_str(k);
            }
            (TokenKind::Time, s) => {
                result.push_str("\x1b[38;5;202m");
                is_colored = true;
                result.push_str(s);
            }
            (TokenKind::Regex, s) => {
                result.push_str("\x1b[38;5;203m");
                is_colored = true;
                result.push_str(s);
            }
            (TokenKind::StringRaw, s) => {
                result.push_str("\x1b[38;5;204m");
                is_colored = true;
                result.push_str(s);
            }
            (TokenKind::StringTemplate, s) => {
                result.push_str("\x1b[38;5;205m");
                is_colored = true;
                result.push_str(s);
            }
            (TokenKind::StringLiteral, s) => {
                result.push_str("\x1b[38;5;208m");
                is_colored = true;

                if let Diagnostic::InvalidStringEscapes(ranges) = diagnostic {
                    let mut last_end = token.range.start();

                    for &range in ranges.iter() {
                        result.push_str(&line[last_end..range.start()]);
                        result.push_str("\x1b[38;5;9m");
                        result.push_str(range.to_str(line));
                        result.push_str("\x1b[38;5;208m");
                        last_end = range.end();
                    }

                    result.push_str(&line[last_end..token.range.end()]);
                } else {
                    result.push_str(s);
                }
            }
            (TokenKind::IntegerLiteral | TokenKind::FloatLiteral, l) => {
                if let Diagnostic::InvalidNumber(e) = diagnostic {
                    result.push_str("\x1b[38;5;9m");
                    result.push_str(e.to_str(line));
                    is_colored = true;
                } else {
                    if is_colored {
                        result.push_str("\x1b[m\x1b[0m");
                        is_colored = false;
                    }
                    result.push_str(l);
                }
            }
            (TokenKind::Symbol, l) => {
                if let Diagnostic::IllegalChar(e) = diagnostic {
                    result.push_str("\x1b[38;5;9m");
                    result.push_str(e.to_str(line));
                    is_colored = true;
                } else {
                    if l == "None" {
                        result.push_str("\x1b[91m");
                        is_colored = true;
                    } else if matches!(l, "echo" | "exit" | "clear" | "cd" | "rm") {
                        result.push_str("\x1b[94m");
                        is_colored = true;
                    } else if is_colored {
                        result.push_str("\x1b[m\x1b[0m");
                        is_colored = false;
                    }
                    result.push_str(l);
                }
            }
            (TokenKind::Whitespace, w) => {
                result.push_str(w);
            }
            (TokenKind::LineBreak, w) => {
                result.push_str(w);
            }
            // (TokenKind::LineContinuation, w) => {
            //     result.push_str(w);
            // }
            (TokenKind::Comment, w) => {
                result.push_str("\x1b[38;5;247m");
                is_colored = true;
                result.push_str(w);
            }
        }
    }
    if diagnostics.len() > tokens.len() {
        for diagnostic in &diagnostics[tokens.len()..] {
            if let Diagnostic::NotTokenized(e) = diagnostic {
                result.push_str("\x1b[38;5;9m");
                result.push_str(e.to_str(line));
                is_colored = true;
            }
        }
    }
    if is_colored {
        result.push_str("\x1b[m\x1b[0m");
    }

    result
}
