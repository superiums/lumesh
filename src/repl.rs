use rustyline::highlight::CmdKind;
use rustyline::validate::ValidationResult;
use rustyline::{
    Changeset,
    Editor,
    // Event::KeySeq,
    Helper,
    KeyEvent,
    completion::{Completer, FilenameCompleter, Pair},
    config::CompletionType,
    error::ReadlineError,
    highlight::Highlighter,
    hint::{Hinter, HistoryHinter},
    history::FileHistory,
    line_buffer::LineBuffer,
    validate::Validator,
};
use rustyline::{Cmd, EditMode, Modifiers, Movement};

use common_macros::hash_set;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::Expression;
use crate::ai::{AIClient, MockAIClient, init_ai};
use crate::cmdhelper::{
    PATH_COMMANDS, should_trigger_cmd_completion, should_trigger_path_completion,
};
use crate::expression::alias::get_alias_tips;
use crate::keyhandler::{LumeAbbrHandler, LumeKeyHandler, LumeMoveHandler};
use crate::modules::get_builtin_tips;
use crate::syntax::{get_ayu_dark_theme, get_dark_theme, get_light_theme, get_merged_theme};

use crate::runtime::check;
use crate::{Environment, highlight, parse_and_eval, prompt::get_prompt_engine};

// use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use std::sync::{Arc, Mutex};

// ANSI 转义码
const DEFAULT: &str = "";
const GREEN_BOLD: &str = "\x1b[1;32m";
// const GRAY: &str = "\x1b[38;5;246m";
// const GRAY2: &str = "\x1b[38;5;249m";
// const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";
// 使用 Arc<Mutex> 包装编辑器

pub fn run_repl(env: &mut Environment) {
    // state::register_signal_handler();

    match env.get("LUME_WELCOME") {
        Some(wel) => {
            println!("{wel}");
            env.undefine("LUME_WELCOME");
        }
        _ => println!("Welcome to Lumesh {}", env!("CARGO_PKG_VERSION")),
    }

    // init_config(env);
    //
    let no_history = match env.get("LUME_NO_HISTORY") {
        Some(Expression::Boolean(t)) => {
            env.undefine("LUME_NO_HISTORY");
            t
        }
        _ => false,
    };
    let history_file = match env.get("LUME_HISTORY_FILE") {
        Some(hf) => hf.to_string(),
        _ => {
            let c_dir = match dirs::cache_dir() {
                Some(c) => c,
                _ => PathBuf::new(),
            };
            #[cfg(unix)]
            let path = c_dir.join(".lume_history");
            #[cfg(windows)]
            let path = c_dir.join("lume_history.log");
            if !path.exists() {
                match std::fs::File::create(&path) {
                    Ok(_) => {}
                    Err(e) => eprint!("Failed to create cache directory: {e}"),
                }
            }
            path.into_os_string().into_string().unwrap()
        }
    };
    // ai config
    let ai_config = env.get("LUME_AI_CONFIG");
    env.undefine("LUME_AI_CONFIG");
    let vi_mode = match env.get("LUME_VI_MODE") {
        Some(Expression::Boolean(true)) => {
            env.undefine("LUME_AI_CONFIG");
            true
        }
        _ => false,
    };

    // theme
    let theme_base = env.get("LUME_THEME");
    env.undefine("LUME_THEME");
    let theme = match theme_base {
        Some(Expression::String(t)) => match t.as_ref() {
            "light" => get_light_theme(),
            "ayu_dark" => get_ayu_dark_theme(),
            _ => get_dark_theme(),
        },
        _ => get_dark_theme(),
    };

    let theme_config = env.get("LUME_THEME_CONFIG");
    env.undefine("LUME_THEME_CONFIG");
    let theme_merged = match theme_config {
        Some(Expression::Map(m)) => get_merged_theme(theme, m.as_ref()),
        _ => theme,
    };

    // 使用 Arc<Mutex> 保护编辑器
    let rl = Arc::new(Mutex::new(new_editor(ai_config, vi_mode, theme_merged)));

    match rl.lock().unwrap().load_history(&history_file) {
        Ok(_) => {}
        Err(e) => println!("No previous history {e}"),
    }

    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

    // 设置信号处理 (Unix 系统)
    // #[cfg(unix)]
    // {
    // let rl_clone = Arc::clone(&rl);
    let running_clone = Arc::clone(&running);
    // if no_history {
    //     ctrlc::set_handler(move || {
    //         running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
    //         // std::process::exit(0);
    //     })
    //     .expect("Error setting Ctrl-C handler");
    // } else {
    // let hist = history_file.clone();
    ctrlc::set_handler(move || {
        running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
        // let _ = rl_clone.lock().unwrap().save_history(&hist);
        // std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");
    // }
    // }

    // =======key binding=======
    // 1. edit
    rl.lock()
        .unwrap()
        .bind_sequence(KeyEvent::ctrl('j'), LumeMoveHandler::new(1));
    rl.lock()
        .unwrap()
        .bind_sequence(KeyEvent::alt('j'), LumeMoveHandler::new(0));
    rl.lock().unwrap().bind_sequence(
        KeyEvent::ctrl('o'),
        Cmd::Replace(Movement::WholeBuffer, Some(String::from(""))),
    );
    let hotkey_sudo = match env.get("LUME_SUDO_CMD") {
        Some(s) => {
            env.undefine("LUME_SUDO_CMD");
            s.to_string()
        }
        _ => "sudo".to_string(),
    };
    rl.lock().unwrap().bind_sequence(
        KeyEvent::alt('s'),
        Cmd::Replace(Movement::BeginningOfLine, Some(hotkey_sudo)),
    );

    // 2. custom hotkey
    let hotkey_modifier = env.get("LUME_HOT_MODIFIER");
    env.undefine("LUME_HOT_MODIFIER");
    let modifier: u8 = match hotkey_modifier {
        Some(Expression::Integer(bits)) => {
            // if bits == 0 {
            //     0
            // } else
            if (bits as u8 & (Modifiers::CTRL | Modifiers::ALT | Modifiers::SHIFT).bits()) == 0 {
                eprintln!("invalid LUME_HOT_MODIFIER {bits}");
                4
            } else {
                bits as u8
            }
        }
        _ => 4,
    };

    let hotkey_config = env.get("LUME_HOT_KEYS");
    env.undefine("LUME_HOT_KEYS");

    if let Some(Expression::Map(keys)) = hotkey_config {
        let mut rl_unlocked = rl.lock().unwrap();
        for (k, cmd) in keys.iter() {
            if let Some(c) = k.chars().next() {
                rl_unlocked.bind_sequence(
                    // KeySeq(vec![
                    //     KeyEvent::alt('z'),
                    KeyEvent::new(c, Modifiers::from_bits_retain(modifier)),
                    // ]),
                    LumeKeyHandler::new(cmd.to_string()),
                );
            }
        }
    }
    // 3. abbr
    let abbr = env.get("LUME_ABBREVIATIONS");
    env.undefine("LUME_ABBREVIATIONS");

    if let Some(Expression::Map(ab)) = abbr {
        let abmap = ab
            .iter()
            .map(|m| (m.0.to_string(), m.1.to_string()))
            .collect::<HashMap<String, String>>();
        rl.lock().unwrap().bind_sequence(
            KeyEvent::new(' ', Modifiers::NONE),
            LumeAbbrHandler::new(abmap),
        );
    }
    // =======key binding end=======

    // main loop
    let pe = get_prompt_engine(
        env.get("LUME_PROMPT_SETTINGS"),
        env.get("LUME_PROMPT_TEMPLATE"),
    );
    env.undefine("LUME_PROMPT_SETTINGS");
    env.undefine("LUME_PROMPT_TEMPLATE");

    // let mut repl_env = env.fork();
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        let prompt = pe.get_prompt();

        // 在锁的保护下执行 readline
        let line = match rl.lock().unwrap().readline(prompt.as_str()) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                // state::set_signal(); // 更新共享状态
                continue;
            }
            // Err(ReadlineError::Signal(sig)) => {
            //     if sig == rustyline::error::Signal::Interrupt {
            //         println!("[Interrupt]");
            //         state::set_signal(); // 更新共享状态
            //     }
            //     continue;
            // }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                continue;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        };

        match line.trim() {
            "" => {}
            "exit" => break,
            "history" => {
                for (i, entry) in rl.lock().unwrap().history().iter().enumerate() {
                    println!("{}{}:{} {}", GREEN_BOLD, i + 1, RESET, entry);
                }
            }
            _ => {
                if parse_and_eval(&line, env)
                // && !no_history
                {
                    match rl.lock().unwrap().add_history_entry(&line) {
                        Ok(_) => {}
                        Err(e) => eprintln!("add history err: {e}"),
                    };
                }
            }
        }
    }

    // 保存历史记录
    if !no_history {
        match rl.lock().unwrap().save_history(&history_file) {
            Ok(_) => {}
            Err(e) => eprintln!("save history err: {e}"),
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
    cmds: HashSet<String>,
}

fn new_editor(
    ai_config: Option<Expression>,
    vi_mode: bool,
    theme: HashMap<String, String>,
) -> Editor<LumeHelper, FileHistory> {
    let config = rustyline::Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::Circular)
        .edit_mode(if vi_mode {
            EditMode::Vi
        } else {
            EditMode::Emacs
        })
        .history_ignore_dups(true)
        .unwrap()
        .build();

    let mut rl = Editor::with_config(config).unwrap_or_else(|_| Editor::new().unwrap());
    let ai = ai_config.map(|ai_cfg| Arc::new(init_ai(ai_cfg)));
    // 预定义命令列表（带权重排序）
    // TODO add builtin cmds
    let mut cmds: HashSet<String> = hash_set! {
        "cd ./".into(),
        "ls -l --color ./".into(),
        "clear".into(),
        "rm ".into(),
        "cp -r".into(),
        "let ".into(),
        "fn ".into(),
        "if ".into(),
        "else {".into(),
        "match ".into(),
        "while (".into(),
        "for i in ".into(),
        "loop {\n".into(),
        "break".into(),
        "return".into(),
        "history".into(),
        "del ".into(),
        "use ".into(),
    };
    cmds.extend(get_builtin_tips());
    cmds.extend(PATH_COMMANDS.lock().unwrap().iter().cloned());
    cmds.extend(get_alias_tips());
    let helper = LumeHelper {
        completer: Arc::new(FilenameCompleter::new()),
        hinter: Arc::new(HistoryHinter::new()),
        highlighter: Arc::new(SyntaxHighlighter::new(theme)),
        ai_client: ai,
        cmds,
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
        match self.detect_completion_type(line, pos) {
            LumeCompletionType::Path => self.path_completion(line, pos, ctx),
            LumeCompletionType::Command => self.cmd_completion(line, pos),
            LumeCompletionType::AI => self.ai_completion(line, pos),
            LumeCompletionType::None => Ok((pos, Vec::new())),
        }
    }

    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str, cl: &mut Changeset) {
        // 直接使用标准替换逻辑
        let end = line.pos();
        line.replace(start..end, elected, cl);
    }
}

// 扩展实现
impl LumeHelper {
    /// 新增补全类型检测
    fn detect_completion_type(&self, line: &str, pos: usize) -> LumeCompletionType {
        if should_trigger_path_completion(line, pos) {
            LumeCompletionType::Path
        } else if should_trigger_cmd_completion(line, pos) {
            LumeCompletionType::Command
        } else if self.should_trigger_ai(line) {
            LumeCompletionType::AI
        } else {
            LumeCompletionType::None
        }
    }

    /// 路径补全逻辑
    fn path_completion(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }

    /// 命令补全逻辑
    fn cmd_completion(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>), ReadlineError> {
        // 计算起始位置
        let input = &line[..pos];
        let start = input.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let prefix = &input[start..];
        // dbg!(&input, &start, &prefix);
        // 过滤以prefix开头的命令
        let cpl_color = self
            .highlighter
            .theme
            .get("completion_cmd")
            .map_or(DEFAULT, |c| c.as_str());
        let mut candidates: Vec<Pair> = self
            .cmds
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| {
                // dbg!(&cmd);
                Pair {
                    display: format!("{cpl_color}{cmd}{RESET}"),
                    replacement: cmd.clone(),
                }
            })
            .collect();
        // 按显示名称的长度升序排序，较短的优先
        candidates.sort_by(|a, b| a.display.len().cmp(&b.display.len()));

        Ok((start, candidates))
    }

    /// AI 补全逻辑
    fn ai_completion(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let prompt = line.trim();
        let suggestion = self
            .ai_client
            .as_ref()
            .and_then(|ai| ai.complete(prompt).ok())
            .unwrap_or_default();

        let pair = Pair {
            display: format!(
                "{}{}{}",
                self.highlighter
                    .theme
                    .get("completion_ai")
                    .map_or(DEFAULT, |c| c.as_str()),
                suggestion,
                RESET
            ), // 保持ANSI颜色
            replacement: suggestion,
        };
        Ok((pos, vec![pair]))
    }

    /// AI补全触发条件
    fn should_trigger_ai(&self, line: &str) -> bool {
        self.ai_client.is_some() && line.split_whitespace().count() > 1
    }
}

// 扩展补全类型枚举
#[derive(Debug, PartialEq)]
enum LumeCompletionType {
    Path,
    Command,
    AI,
    None,
}

impl Validator for LumeHelper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext<'_>,
    ) -> rustyline::Result<ValidationResult> {
        // self.validator.validate(ctx)
        // check_balanced(ctx.input())
        if ctx.input().ends_with(" \\") {
            return Ok(ValidationResult::Incomplete);
        }
        if ctx.input().ends_with("\n\n") || check(ctx.input()) {
            return Ok(ValidationResult::Valid(None));
        }
        Ok(ValidationResult::Incomplete)
    }
}

impl Highlighter for LumeHelper {
    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        self.highlighter.highlight_hint(hint)
    }
    // fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
    //     &'s self,
    //     prompt: &'p str,
    //     default: bool,
    // ) -> Cow<'b, str> {
    //     self.highlighter.highlight_prompt(prompt, default)
    // }
    // fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
    //     MatchingBracketHighlighter::highlight_char(line, pos, kind)
    // }
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
                if matches!(ch, ';' | '|' | '(' | '{' | '`' | '\n') {
                    segment.clear();
                } else if segment.is_empty() && ch.is_ascii_whitespace() {
                } else {
                    segment.push(ch);
                }
                if i == pos {
                    break;
                }
            }
        }
        // 仅当有有效片段时进行匹配
        if !segment.is_empty() {
            // 按权重排序匹配结果
            let mut matches: Vec<_> = self
                .cmds
                .iter()
                .filter(|cmd| cmd.starts_with(&segment))
                .collect();

            // 权重降序, 较短的优先
            matches.sort_by(|a, b| a.len().cmp(&b.len()));
            // dbg!(&matches);
            if let Some(matched) = matches.first() {
                let suffix = &matched[segment.len()..];
                // dbg!(&segment, &segment.len(), &matched, &suffix, &suffix.len());
                if !suffix.is_empty() {
                    return Some(suffix.to_string());
                }
            }
        }

        // 无匹配时回退默认提示
        self.hinter.hint(line, pos, ctx)
    }
}

struct SyntaxHighlighter {
    theme: HashMap<String, String>,
}

impl SyntaxHighlighter {
    pub fn new(theme: HashMap<String, String>) -> Self {
        Self { theme }
    }
}
impl Highlighter for SyntaxHighlighter {
    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        let _s = (line, pos, kind);
        kind != CmdKind::MoveCursor
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let mut parts = line.splitn(2, |c: char| c.is_whitespace());
        let cmd = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        if cmd.is_empty() {
            return Cow::Borrowed(line);
        }

        let (color, is_valid) = if is_valid_command(cmd) {
            (
                self.theme
                    .get("command_valid")
                    .map_or(DEFAULT, |c| c.as_str()),
                true,
            )
        // } else if !cmd.is_empty() {
        //     (RED, false)
        } else {
            // 无命令，直接返回语法高亮
            return Cow::Owned(highlight(line, &self.theme));
        };

        // 高亮命令部分，剩余部分调用 syntax_highlight
        let highlighted_rest = highlight(rest, &self.theme);
        let colored_line = if is_valid {
            format!("{color}{cmd}{RESET} {highlighted_rest}")
        } else {
            format!("{color}{cmd}{RESET} {highlighted_rest}")
        };
        Cow::Owned(colored_line)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // dbg!(&hint);
        // 如果提示为空或已经包含颜色代码，直接返回借用
        if hint.is_empty() || hint.contains('\x1b') {
            return Cow::Borrowed(hint);
        }
        Cow::Owned(format!(
            "{}{}{}",
            self.theme.get("hint").map_or(DEFAULT, |c| c.as_str()),
            hint,
            RESET
        ))
    }

    // fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
    //     &'s self,
    //     prompt: &'p str,
    //     default: bool,
    // ) -> Cow<'b, str> {
    //     // dbg!(&prompt);
    //     let _ = default;
    //     // Borrowed(prompt)
    //     Cow::Owned(format!("{}{}{}", GREEN_BOLD, prompt, RESET))
    // }
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

// fn readline(prompt: impl ToString, rl: &mut Editor<LumeHelper, FileHistory>) -> String {
//     let prompt = prompt.to_string();
//     loop {
//         // if let Some(helper) = rl.helper_mut() {
//         //     helper.set_prompt(&prompt);
//         // }

//         match rl.readline(&strip_ansi_escapes(&prompt)) {
//             Ok(line) => return line,
//             Err(ReadlineError::Interrupted) => return String::new(),
//             Err(ReadlineError::Eof) => exit(0),
//             Err(err) => eprintln!("Error: {:?}", err),
//         }
//     }
// }

// pub fn read_user_input(prompt: impl ToString) -> String {
//     let mut rl = new_editor(None);
//     readline(prompt, &mut rl)
// }

fn is_valid_command(cmd: &str) -> bool {
    PATH_COMMANDS.lock().unwrap().contains(cmd)
}
