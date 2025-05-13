use rustyline::validate::ValidationResult;
use rustyline::{
    Changeset, Editor, Helper,
    completion::{Completer, FilenameCompleter, Pair},
    config::CompletionType,
    error::ReadlineError,
    highlight::Highlighter,
    hint::{Hinter, HistoryHinter},
    history::FileHistory,
    line_buffer::LineBuffer,
    validate::Validator,
};
use std::borrow::Cow;
use std::process::exit;

use crate::Expression;
use crate::ai::{AIClient, MockAIClient, init_ai};
use crate::cmdhelper::{
    PATH_COMMANDS, should_trigger_cmd_completion, should_trigger_path_completion,
};
use crate::runtime::check;
use crate::{Environment, parse_and_eval, prompt::get_prompt_engine, syntax_highlight};

// use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use std::sync::{Arc, Mutex};

// ANSI 转义码
const GREEN_BOLD: &str = "\x1b[1;32m";
// const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

// 使用 Arc<Mutex> 包装编辑器
pub fn run_repl(env: &mut Environment) {
    match env.get("LUME_WELCOME") {
        Some(wel) => println!("{}", wel.to_string()),
        _ => println!("Welcome to Lumesh {}", env!("CARGO_PKG_VERSION")),
    }
    // init_config(env);
    //
    let no_history = match env.get("LUME_NO_HISTORY") {
        Some(Expression::Boolean(t)) => t,
        _ => false,
    };
    let history_file = match env.get("LUME_HISTORY_FILE") {
        Some(hf) => hf.to_string(),
        _ => {
            if let Some(c_dir) = dirs::cache_dir() {
                c_dir
                    .join("lume_history")
                    .into_os_string()
                    .into_string()
                    .unwrap()
            } else {
                eprintln!("please config LUME_HISTORY_FILE");
                "lume_history".into()
            }
        }
    };
    let ai_config = env.get("LUME_AI_CONFIG");
    // let enable_ai = match env.get("LUME_AI_CONFIG") {
    //     Some(_) => true,
    //     _ => false,
    // };

    // 使用 Arc<Mutex> 保护编辑器
    let rl = Arc::new(Mutex::new(new_editor(ai_config)));

    if rl.lock().unwrap().load_history(&history_file).is_err() {
        println!("No previous history");
    }

    let pe = get_prompt_engine();
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

    // 设置信号处理 (Unix 系统)
    #[cfg(unix)]
    {
        let rl_clone = Arc::clone(&rl);
        let running_clone = Arc::clone(&running);
        let hist = history_file.clone();
        if no_history {
            ctrlc::set_handler(move || {
                running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
                std::process::exit(0);
            })
            .expect("Error setting Ctrl-C handler");
        } else {
            ctrlc::set_handler(move || {
                running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
                let _ = rl_clone.lock().unwrap().save_history(&hist);
                std::process::exit(0);
            })
            .expect("Error setting Ctrl-C handler");
        }
    }

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        let prompt = pe.get_prompt();

        // 在锁的保护下执行 readline
        let line = match rl.lock().unwrap().readline(prompt.as_str()) {
            Ok(line) => line,
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
        };

        match line.trim() {
            "" => {}
            "exit" => break,
            "history" => {
                for (i, entry) in rl.lock().unwrap().history().iter().enumerate() {
                    println!("{}: {}", i + 1, entry);
                }
            }
            _ => {
                if parse_and_eval(&line, env)
                // && !no_history
                {
                    match rl.lock().unwrap().add_history_entry(&line) {
                        Ok(_) => {}
                        Err(e) => eprintln!("add history err: {}", e),
                    };
                }
            }
        }
    }

    // 保存历史记录
    if !no_history {
        match rl.lock().unwrap().save_history(&history_file) {
            Ok(_) => {}
            Err(e) => eprintln!("save history err: {}", e),
        };
    }
}

// 确保 helper 也是线程安全的
#[derive(Clone)]
struct LumeHelper {
    completer: Arc<FilenameCompleter>,
    hinter: Arc<HistoryHinter>,
    highlighter: Arc<SyntaxHighlighter>,
    ai_client: Option<Arc<MockAIClient>>,
}

fn new_editor(ai_config: Option<Expression>) -> Editor<LumeHelper, FileHistory> {
    let config = rustyline::Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::Circular)
        .build();

    let mut rl = Editor::with_config(config).unwrap_or_else(|_| Editor::new().unwrap());
    let ai = ai_config.map(|ai_cfg| Arc::new(init_ai(ai_cfg)));
    let helper = LumeHelper {
        completer: Arc::new(FilenameCompleter::new()),
        hinter: Arc::new(HistoryHinter::new()),
        highlighter: Arc::new(SyntaxHighlighter),
        ai_client: ai,
    };
    rl.set_helper(Some(helper));
    rl
}

impl Helper for LumeHelper {}
// impl LumeHelper {
//     fn set_prompt(&mut self, prompt: impl ToString) {
//         self.colored_prompt = prompt.to_string();
//     }

//     fn update_env(&mut self, env: &Environment) {
//         self.env = env.clone();
//     }
// }

impl Completer for LumeHelper {
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
            Ok((start, completions))
        } else if should_trigger_cmd_completion(line, pos) {
            return Ok(generate_cmd_hints(line, pos));
        } else if let Some(ai) = &self.ai_client {
            // After the first token, use AI completion
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() > 1 {
                let prompt = line.trim();
                match ai.complete(prompt) {
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
        } else {
            return Ok((pos, Vec::new()));
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

impl Validator for LumeHelper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext<'_>,
    ) -> rustyline::Result<ValidationResult> {
        // self.validator.validate(ctx)
        // check_balanced(ctx.input())
        if ctx.input().ends_with("\n\n") || check(ctx.input()) {
            return Ok(ValidationResult::Valid(None));
        };
        Ok(ValidationResult::Incomplete)
    }
}

impl Highlighter for LumeHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }
}

// struct InputValidator;

// impl Validator for InputValidator {
//     fn validate(
//         &self,
//         ctx: &mut rustyline::validate::ValidationContext<'_>,
//     ) -> rustyline::Result<ValidationResult> {
//         // dbg!(
//         //     ctx.input(),
//         //     ctx.input().ends_with("\n\n"),
//         //     check_balanced(ctx.input()),
//         //     check(ctx.input())
//         // );

//     }
// }
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
impl Hinter for LumeHelper {
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
        let cmds = [
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
        let mut parts = line.splitn(2, |c: char| c.is_whitespace());
        let cmd = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");

        let (color, is_valid) = if is_valid_command(cmd) {
            (GREEN_BOLD, true)
        // } else if !cmd.is_empty() {
        //     (RED, false)
        } else {
            // 无命令，直接返回语法高亮
            return Cow::Owned(syntax_highlight(line));
        };

        // 高亮命令部分，剩余部分调用 syntax_highlight
        let highlighted_rest = syntax_highlight(rest);
        let colored_line = if is_valid {
            format!("{}{}{} {}", color, cmd, RESET, highlighted_rest)
        } else {
            format!("{}{}{} {}", color, cmd, RESET, highlighted_rest)
        };
        Cow::Owned(colored_line)
    }
}
// fn check_balanced(input: &str) -> bool {
//     let mut stack = Vec::new();
//     for c in input.chars() {
//         match c {
//             '(' | '[' | '{' => stack.push(c),
//             ')' => {
//                 if stack.pop() != Some('(') {
//                     return false;
//                 }
//             }
//             ']' => {
//                 if stack.pop() != Some('[') {
//                     return false;
//                 }
//             }
//             '}' => {
//                 if stack.pop() != Some('{') {
//                     return false;
//                 }
//             }
//             _ => {}
//         }
//     }
//     stack.is_empty()
// }

// pub fn run_repl(env: &mut Environment) -> Result<(), LmError> {
//     println!("Rustyline Enhanced CLI (v15.0.0)");
//     init_config(env); // 调用 REPL 初始化

//     let mut rl = new_editor();
//     if rl.load_history(HISTORY_FILE).is_err() {
//         println!("No previous history");
//     }
//     // let mut lines = vec![];

//     // 示例：设置自定义模板 (可选)
//     let pe = get_prompt_engine();

//     loop {
//         // let is_incomplete = rl
//         //     .helper()
//         //     .map(|h| *h.is_incomplete.borrow())
//         //     .unwrap_or(false);
//         // dbg!(is_incomplete);
//         // // 动态设置提示符
//         // let prompt = if is_incomplete {
//         //     pe.get_incomplete_prompt()
//         // } else {
//         //     pe.get_prompt()
//         // };
//         // let prompt = get_prompt(env);
//         // let prompt = prompt_engine.get_prompt();
//         let prompt = pe.get_prompt();

//         match rl.readline(prompt.as_str()) {
//             Ok(line) => {
//                 match line.trim() {
//                     "" => {}
//                     // "cd" => {
//                     //     env::set_current_dir(path)?;
//                     // }
//                     "exit" => break,
//                     "history" => {
//                         for (i, entry) in rl.history().iter().enumerate() {
//                             println!("{}: {}", i + 1, entry);
//                         }
//                     }
//                     // main
//                     _ => {
//                         dbg!(&line);
//                         if parse_and_eval(&line, env) {
//                             rl.add_history_entry(&line)
//                                 .map_err(|_| LmError::CustomError("add history err".into()))?;
//                         };
//                     }
//                 }
//             }
//             Err(ReadlineError::Interrupted) => {
//                 println!("CTRL-C");
//                 break;
//             }
//             Err(ReadlineError::Eof) => {
//                 println!("CTRL-D");
//                 break;
//             }
//             Err(err) => {
//                 println!("Error: {:?}", err);
//                 break;
//             }
//         }
//     }

//     rl.save_history(HISTORY_FILE)
//         .map_err(|_| LmError::CustomError("save histroy err".into()))?;
//     Ok(())
// }

// fn new_editor() -> Editor<LumeHelper, FileHistory> {
//     let config = rustyline::Config::builder()
//         .history_ignore_space(true)
//         .completion_type(CompletionType::List)
//         .build();

//     let mut rl = Editor::with_config(config).unwrap_or(Editor::new().unwrap());

//     let helper = LumeHelper {
//         completer: FilenameCompleter::new(),
//         hinter: HistoryHinter::new(),
//         // validator: InputValidator,
//         highlighter: SyntaxHighlighter,
//         // colored_prompt: ">>>".into(),
//         // env: env.clone(),
//         // is_incomplete: RefCell::new(false),
//     };
//     rl.set_helper(Some(helper));
//     rl
// }

fn readline(prompt: impl ToString, rl: &mut Editor<LumeHelper, FileHistory>) -> String {
    let prompt = prompt.to_string();
    loop {
        // if let Some(helper) = rl.helper_mut() {
        //     helper.set_prompt(&prompt);
        // }

        match rl.readline(&strip_ansi_escapes(&prompt)) {
            Ok(line) => return line,
            Err(ReadlineError::Interrupted) => return String::new(),
            Err(ReadlineError::Eof) => exit(0),
            Err(err) => eprintln!("Error: {:?}", err),
        }
    }
}

pub fn read_user_input(prompt: impl ToString) -> String {
    let mut rl = new_editor(None);
    readline(prompt, &mut rl)
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
