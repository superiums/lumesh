use rustyline::validate::ValidationResult;
use rustyline::{
    Changeset, Editor, Helper,
    completion::{Completer, FilenameCompleter, Pair},
    config::CompletionType,
    error::ReadlineError,
    highlight::Highlighter,
    hint::{Hint, Hinter, HistoryHinter},
    history::{FileHistory, History, SearchDirection},
    line_buffer::LineBuffer,
    validate::Validator,
};
use std::borrow::Cow;
use std::fs;
use std::path::Path;
use std::process::exit;

use crate::{Environment, Error, parse_and_eval};

pub struct MyHelper {
    completer: FilenameCompleter,
    hinter: HistoryHinter,
    validator: InputValidator,
    highlighter: SyntaxHighlighter,
    colored_prompt: String,
    env: Environment,
}

impl Helper for MyHelper {}
impl MyHelper {
    fn set_prompt(&mut self, prompt: impl ToString) {
        self.colored_prompt = prompt.to_string();
    }

    fn update_env(&mut self, env: &Environment) {
        self.env = env.clone();
    }
}

impl Completer for MyHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
        let (start, mut completions) = self.completer.complete(line, pos, ctx)?;

        if line.trim().starts_with("cargo") {
            completions.extend(vec![
                Pair {
                    display: "cargo build".to_string(),
                    replacement: "cargo build".to_string(),
                },
                Pair {
                    display: "cargo run".to_string(),
                    replacement: "cargo run".to_string(),
                },
            ]);
        }

        Ok((start, completions))
    }

    // 支持部分补全
    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str, cl: &mut Changeset) {
        let pos = line.pos();
        let item = elected.split_whitespace().last().unwrap_or("");
        line.update(item, pos, cl);
        // let end = line.pos();
        // line.replace(start..end, elected, cl);
    }
}

impl Validator for MyHelper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext<'_>,
    ) -> rustyline::Result<ValidationResult> {
        self.validator.validate(ctx)
    }
}

impl Highlighter for MyHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }
}

struct InputValidator;

impl Validator for InputValidator {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext<'_>,
    ) -> rustyline::Result<ValidationResult> {
        if !check_balanced(ctx.input()) {
            return Ok(ValidationResult::Incomplete);
        }
        Ok(ValidationResult::Valid(None))
    }
}
// 实现历史提示
// Define a concrete Hint type for HistoryHinter
pub struct HistoryHint(String);

impl Hint for HistoryHint {
    fn display(&self) -> &str {
        &self.0
    }

    fn completion(&self) -> Option<&str> {
        Some(&self.0)
    }
}

impl Hinter for MyHelper {
    type Hint = HistoryHint;

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<HistoryHint> {
        Some(HistoryHint("hinter here".to_string()))
        // self.hinter.hint(line, pos, ctx)
    }
}

struct SyntaxHighlighter;

impl Highlighter for SyntaxHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let mut result = line.to_string();
        if line.contains("fn ") {
            result = result.replace("fn ", "\x1b[35mfn\x1b[0m ");
        }
        Cow::Owned(result)
    }
}

fn check_balanced(input: &str) -> bool {
    let mut stack = Vec::new();
    for c in input.chars() {
        match c {
            '(' | '[' | '{' => stack.push(c),
            ')' => {
                if stack.pop() != Some('(') {
                    return false;
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return false;
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return false;
                }
            }
            _ => {}
        }
    }
    stack.is_empty()
}

fn load_completion_files() -> Vec<String> {
    let mut completions = Vec::new();

    if let Ok(entries) = fs::read_dir("/usr/share/bash-completion/completions") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                completions.push(name.to_string());
            }
        }
    }

    if let Ok(entries) = fs::read_dir(dirs::home_dir().unwrap().join(".config/fish/completions")) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                completions.push(name.to_string());
            }
        }
    }

    completions
}

pub fn run_repl(env: &mut Environment) -> Result<(), Error> {
    println!("Rustyline Enhanced CLI (v15.0.0)");

    let completions = load_completion_files();
    println!("Loaded {} completion files", completions.len());

    let mut rl = new_editor(env);
    if rl.load_history(".history.txt").is_err() {
        println!("No previous history");
    }

    loop {
        match rl.readline(">> ") {
            Ok(line) => {
                rl.add_history_entry(&line);
                println!("Line: {}", line);

                if line.trim() == "exit" {
                    break;
                } else if line.trim() == "history" {
                    for (i, entry) in rl.history().iter().enumerate() {
                        println!("{}: {}", i + 1, entry);
                    }
                }

                parse_and_eval(&line, env);
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    rl.save_history(".history.txt")
        .map_err(|_| Error::CustomError("readline err".into()))?;
    Ok(())
}

pub fn new_editor(env: &mut Environment) -> Editor<MyHelper, FileHistory> {
    let config = rustyline::Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::Circular)
        .build();

    let mut rl = Editor::with_config(config).unwrap_or(Editor::new().unwrap());

    let helper = MyHelper {
        completer: FilenameCompleter::new(),
        hinter: HistoryHinter::new(),
        validator: InputValidator,
        highlighter: SyntaxHighlighter,
        colored_prompt: ">>>".into(),
        env: env.clone(),
    };
    rl.set_helper(Some(helper));
    rl
}

pub fn readline(prompt: impl ToString, rl: &mut Editor<MyHelper, FileHistory>) -> String {
    let prompt = prompt.to_string();
    loop {
        if let Some(helper) = rl.helper_mut() {
            helper.set_prompt(&prompt);
        }

        match rl.readline(&strip_ansi_escapes(&prompt)) {
            Ok(line) => return line,
            Err(ReadlineError::Interrupted) => return String::new(),
            Err(ReadlineError::Eof) => exit(0),
            Err(err) => eprintln!("Error: {:?}", err),
        }
    }
}

pub fn strip_ansi_escapes(text: impl ToString) -> String {
    let text = text.to_string();
    let mut result = String::new();
    let mut is_in_escape = false;
    for ch in text.chars() {
        if ch == '\x1b' {
            is_in_escape = true;
        } else if is_in_escape && ch == 'm' {
            is_in_escape = false;
        } else if !is_in_escape {
            result.push(ch);
        }
    }
    result
}
