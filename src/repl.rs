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

use crate::cmdhelper::{
    AI_CLIENT, PATH_COMMANDS, should_trigger_cmd_completion, should_trigger_path_completion,
};
use crate::runtime::check;
use crate::{Environment, Error, parse_and_eval, syntax_highlight};

use lazy_static::lazy_static;
use rustyline::history::DefaultHistory;
use rustyline::validate::ValidationContext;
use rustyline::{Config, Context};
use std::collections::HashSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
// use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const HISTORY_FILE: &str = "/tmp/lume-history";
// ANSI 转义码
const GREEN_BOLD: &str = "\x1b[1;32m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

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
        if should_trigger_path_completion(line, pos) {
            // 路径
            let (start, completions) = self.completer.complete(line, pos, ctx)?;
            return Ok((start, completions));
        } else if should_trigger_cmd_completion(line, pos) {
            return Ok(generate_cmd_hints(line, pos));
        } else {
            // After the first token, use AI completion
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() > 1 {
                let prompt = line.trim();
                match AI_CLIENT.complete(prompt) {
                    Ok(suggestion) => {
                        let pair = Pair {
                            display: format!("\x1b[34m{}\x1b[0m", suggestion),
                            replacement: suggestion,
                        };
                        return Ok((pos, vec![pair]));
                    }
                    Err(_) => return Ok((pos, Vec::new())),
                }
            } else {
                return Ok((pos, Vec::new()));
            }
        }
    }

    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str, cl: &mut Changeset) {
        // 直接使用标准替换逻辑
        let end = line.pos();
        line.replace(start..end, elected, cl);
    }
}

fn generate_cmd_hints(line: &str, pos: usize) -> (usize, Vec<Pair>) {
    // 获取PATH_COMMANDS的锁
    let path_commands = PATH_COMMANDS.lock().unwrap();

    // 计算起始位置
    let input = &line[..pos];
    let start = input.rfind(' ').map(|i| i + 1).unwrap_or(0);
    let prefix = &input[start..];

    // 过滤以prefix开头的命令
    let mut candidates: Vec<Pair> = path_commands
        .iter()
        .filter(|cmd| cmd.starts_with(prefix))
        .map(|cmd| Pair {
            display: cmd.clone(),
            replacement: cmd.clone(),
        })
        .collect();

    // 按显示名称的长度升序排序
    candidates.sort_by(|a, b| a.display.len().cmp(&b.display.len()));

    (start, candidates)
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
        if !check_balanced(ctx.input()) || !check(ctx.input()) {
            return Ok(ValidationResult::Incomplete);
        }
        Ok(ValidationResult::Valid(None))
    }
}
// 实现历史提示
// Define a concrete Hint type for HistoryHinter
// pub struct HistoryHint(String);

// impl Hint for HistoryHint {
//     fn display(&self) -> &str {
//         &self.0
//     }

//     fn completion(&self) -> Option<&str> {
//         Some(&self.0)
//     }
// }
impl Hinter for MyHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        // 提取光标前的连续非分隔符片段
        let mut segment = String::new();
        if !line.is_empty() {
            for (i, ch) in line.chars().enumerate() {
                // 扩展分隔符列表（根据需要调整）
                if ch.is_whitespace()
                    || matches!(ch, ';' | '\'' | '(' | ')' | '{' | '}' | '"' | '\\' | '`')
                {
                    segment.clear();
                } else {
                    segment.push(ch);
                }
                if i == pos {
                    break;
                }
            }
        }

        // 预定义命令列表（带权重排序）
        let cmds = vec![
            ("cd", 10),
            ("ls", 9),
            ("clear", 8),
            ("exit 0", 7),
            ("rm -ri", 6),
            ("cp -r", 5),
            ("head", 4),
            ("tail", 3),
        ];

        // 仅当有有效片段时进行匹配
        if !segment.is_empty() {
            // 按权重排序匹配结果
            let mut matches: Vec<_> = cmds
                .iter()
                .filter(|(cmd, _)| cmd.starts_with(&segment))
                .collect();

            matches.sort_by(|a, b| b.1.cmp(&a.1)); // 权重降序

            if let Some((matched, _)) = matches.first() {
                let suffix = &matched[segment.len()..];
                if !suffix.is_empty() {
                    return Some(suffix.to_string());
                }
            }
        }

        // 无匹配时回退默认提示
        self.hinter.hint(line, pos, ctx)
    }
}

struct SyntaxHighlighter;

impl Highlighter for SyntaxHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        // 提取第一个单词作为命令
        let cmd = line.split_whitespace().next().unwrap_or("");

        if is_valid_command(cmd) {
            // 有效命令：绿色粗体
            Cow::Owned(format!("{}{}{}", GREEN_BOLD, line, RESET))
        } else if !cmd.is_empty() {
            // 无效命令：红色
            Cow::Owned(format!("{}{}{}", RED, line, RESET))
        } else {
            let result = syntax_highlight(line);
            Cow::Owned(result)
        }
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

pub fn run_repl(env: &mut Environment) -> Result<(), Error> {
    println!("Rustyline Enhanced CLI (v15.0.0)");

    let mut rl = new_editor(env);
    if rl.load_history(HISTORY_FILE).is_err() {
        println!("No previous history");
    }

    // let mut lines = vec![];

    loop {
        let prompt = get_prompt(env);
        match rl.readline(prompt.as_str()) {
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
                dbg!(&line);

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

    rl.save_history(HISTORY_FILE)
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
fn get_prompt(env: &mut Environment) -> String {
    let cwd = env.get_cwd();
    return format!("{} >> ", cwd);
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
fn is_valid_command(cmd: &str) -> bool {
    PATH_COMMANDS.lock().unwrap().contains(cmd)
}
