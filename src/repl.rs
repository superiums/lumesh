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

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::ai::{AIClient, MockAIClient, init_ai};
use crate::cmdhelper::{
    LumeCompletionType, collect_command_with_prefix, detect_completion_type, is_valid_command,
};
use crate::completion::ParamCompleter;
use crate::keyhandler::{LumeAbbrHandler, LumeKeyHandler, LumeMoveHandler};
use crate::syntax::{get_ayu_dark_theme, get_dark_theme, get_light_theme, get_merged_theme};
use crate::{Expression, childman};

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

    // complition
    let completion_dir = match env.get("LUME_COMPLETION_DIR") {
        Some(Expression::String(c)) => c,
        _ => String::from("/usr/share/lumesh/completion"),
    };
    env.undefine("LUME_COMPLETION_DIR");

    // 使用 Arc<Mutex> 保护编辑器
    let cfg = EditorConfig {
        ai_config,
        vi_mode,
        theme: theme_merged,
        completion_dir,
    };
    let rl = Arc::new(Mutex::new(new_editor(cfg)));

    match rl.lock().unwrap().load_history(&history_file) {
        Ok(_) => {}
        Err(e) => println!("No previous history {e}"),
    }

    // let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

    // 设置信号处理 (Unix 系统)
    // let running_clone = Arc::clone(&running);
    ctrlc::set_handler(move || {
        // running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
        println!("^C");
        if childman::kill_child() {
            childman::clear_child();
        }
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
                eprintln!("invalid LUME_HOT_MODIFIER {bits}, fallback to `Alt`");
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
                    KeyEvent::new(c, Modifiers::from_bits_retain(modifier)),
                    // \n is for skip CFM
                    LumeKeyHandler::new(cmd.to_string() + "\n"),
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
    // while running.load(std::sync::atomic::Ordering::SeqCst) {
    loop {
        let prompt = pe.get_prompt();

        // 在锁的保护下执行 readline
        let line = match rl.lock().unwrap().readline(prompt.as_str()) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                // state::set_signal(); // 更新共享状态
                continue;
            }
            Err(ReadlineError::Signal(sig)) => {
                #[cfg(unix)]
                if sig == rustyline::error::Signal::Interrupt {
                    println!("[Interrupt]");
                    if childman::kill_child() {
                        childman::clear_child();
                    }
                    // state::set_signal(); // 更新共享状态
                }
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                continue;
            }
            Err(err) => {
                println!("Error: {err:?}");
                continue;
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
    file_completer: Arc<FilenameCompleter>,
    hinter: Arc<HistoryHinter>,
    highlighter: Arc<SyntaxHighlighter>,
    ai_client: Option<Arc<MockAIClient>>,
    param_completer: Arc<ParamCompleter>,
}

struct EditorConfig {
    ai_config: Option<Expression>,
    vi_mode: bool,
    theme: HashMap<String, String>,
    completion_dir: String,
}
fn new_editor(cfg: EditorConfig) -> Editor<LumeHelper, FileHistory> {
    let config = rustyline::Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(if cfg.vi_mode {
            EditMode::Vi
        } else {
            EditMode::Emacs
        })
        .history_ignore_dups(true)
        .unwrap()
        .build();

    let mut rl = Editor::with_config(config).unwrap_or_else(|_| Editor::new().unwrap());
    let ai = cfg.ai_config.map(|ai_cfg| Arc::new(init_ai(ai_cfg)));

    let helper = LumeHelper {
        file_completer: Arc::new(FilenameCompleter::new()),
        hinter: Arc::new(HistoryHinter::new()),
        highlighter: Arc::new(SyntaxHighlighter::new(cfg.theme)),
        ai_client: ai,
        param_completer: Arc::new(ParamCompleter::new(cfg.completion_dir)),
    };
    rl.set_helper(Some(helper));
    rl
}

impl Helper for LumeHelper {}

impl Completer for LumeHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
        match detect_completion_type(line, pos, self.ai_client.is_some()) {
            (LumeCompletionType::Path, _) => self.file_completer.complete(line, pos, ctx),
            (LumeCompletionType::Command, section_pos) => {
                self.cmd_completion(line, pos, section_pos)
            }
            (LumeCompletionType::Param, section_pos) => {
                self.param_completion(line, pos, section_pos, ctx)
            }
            (LumeCompletionType::AI, section_pos) => self.ai_completion(line, section_pos),
            (LumeCompletionType::None, _) => Ok((pos, Vec::new())),
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
    /// 命令补全逻辑
    fn cmd_completion(
        &self,
        line: &str,
        pos: usize,
        section_start: usize,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let prefix = &line[section_start..pos];
        // dbg!(&input, &start, &prefix);
        // 过滤以prefix开头的命令
        let cpl_color = self
            .highlighter
            .theme
            .get("completion_cmd")
            .map_or(DEFAULT, |c| c.as_str());
        let mut candidates: Vec<Pair> = collect_command_with_prefix(prefix)
            .iter()
            .map(|cmd| {
                // dbg!(&cmd);
                Pair {
                    display: format!("{cpl_color}{cmd}{RESET}"),
                    replacement: cmd.to_string(),
                }
            })
            .collect();
        // 按显示名称的长度升序排序，较短的优先
        candidates.sort_by(|a, b| a.display.len().cmp(&b.display.len()));

        Ok((section_start, candidates))
    }

    /// 参数补全逻辑
    fn param_completion(
        &self,
        line: &str,
        pos: usize,
        section_start: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let cmd_section = &line[section_start..pos];

        let tokens = cmd_section
            .split_whitespace()
            .map(|s| s)
            .collect::<Vec<_>>();

        if let Some((command, tokens)) = tokens.split_first() {
            let current_token = if let Some(last_space) = cmd_section.rfind(' ') {
                &cmd_section[last_space + 1..]
            } else {
                ""
            };

            let start = if let Some(last_space) = cmd_section.rfind(' ') {
                last_space + 1
            } else {
                0
            };

            let args = if current_token.is_empty() {
                tokens
            } else {
                tokens[..tokens.len() - 1].as_ref()
            };

            let (mut candidates, trig_file) =
                self.param_completer
                    .get_completions_for_context(command, args, current_token);

            if trig_file {
                return self.file_completer.complete(line, section_start, ctx);
            }
            // Sort by priority and then by length
            candidates.sort_by(|a, b| a.replacement.cmp(&b.replacement));

            return Ok((start, candidates));
        }
        Ok((pos, Vec::new()))
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
            let mut matches: Vec<_> = collect_command_with_prefix(&segment);

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
