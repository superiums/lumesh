#![allow(clippy::wildcard_in_or_patterns)]

mod binary;

use dune::{parse_script, Diagnostic, Environment, Error, Expression, SyntaxError, TokenKind};

use clap::{arg, crate_authors, crate_description, App};

use rustyline::completion::{Completer, FilenameCompleter, Pair as PairComplete};
use rustyline::config::OutputStreamType;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{
    MatchingBracketValidator, ValidationContext, ValidationResult, Validator,
};
use rustyline::{error::ReadlineError, Editor};
use rustyline::{CompletionType, Config, Context, EditMode};
use rustyline_derive::Helper;

use os_info::Type;

use std::{
    borrow::Cow::{self, Borrowed, Owned},
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex},
};

#[rustfmt::skip]
const INTRO_PRELUDE: &str = include_str!(".intro-dune-prelude");
#[rustfmt::skip]
const DEFAULT_PRELUDE: &str = include_str!(".default-dune-prelude");

/// Get the path to the stored history of dune commands.
fn get_history_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".dune-history"))
}
/** 初始化REPL环境并设置提示符
 */
fn new_editor(env: &Environment) -> Editor<DuneHelper> {
    let config = Config::builder()
        .history_ignore_dups(true)
        .history_ignore_space(true)
        .auto_add_history(false)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .check_cursor_position(true)
        .output_stream(OutputStreamType::Stdout)
        .build();

    let mut rl = Editor::with_config(config);
    let h = DuneHelper {
        completer: FilenameCompleter::new(),
        hinter: HistoryHinter {},
        validator: MatchingBracketValidator::new(),
        colored_prompt: "".to_string(),
        env: env.clone(),
    };
    rl.set_helper(Some(h));
    rl
}

fn strip_ansi_escapes(text: impl ToString) -> String {
    let text = text.to_string();

    let mut result = String::new();
    let mut is_in_escape = false;
    for ch in text.chars() {
        // If this is the start of a new escape
        if ch == '\x1b' {
            is_in_escape = true;
        // If this is the end of an escape
        } else if is_in_escape && ch == 'm' {
            is_in_escape = false;
        // If this is any other sort of text
        } else if !is_in_escape {
            result.push(ch);
        }
    }

    result
}

/** 读取用户输入 */
fn readline(prompt: impl ToString, rl: &mut Editor<DuneHelper>) -> String {
    let prompt = prompt.to_string();
    loop {
        // This MUST be called to update the prompt.
        if let Some(helper) = rl.helper_mut() {
            helper.set_prompt(&prompt);
        }

        match rl.readline(&strip_ansi_escapes(&prompt)) {
            Ok(line) => return line,
            Err(ReadlineError::Interrupted) => {
                return String::new();
            }
            Err(ReadlineError::Eof) => exit(0),
            Err(err) => {
                eprintln!("Error: {:?}", err);
            }
        }
    }
}

#[derive(Helper)]
struct DuneHelper {
    completer: FilenameCompleter,
    hinter: HistoryHinter,
    colored_prompt: String,
    validator: MatchingBracketValidator,
    env: Environment,
}

impl DuneHelper {
    /// This method MUST be called to update the prompt.
    /// If this method is not called, the prompt will not
    /// update.
    fn set_prompt(&mut self, prompt: impl ToString) {
        self.colored_prompt = prompt.to_string();
    }

    fn update_env(&mut self, env: &Environment) {
        self.env = env.clone();
    }
}

impl Completer for DuneHelper {
    type Candidate = PairComplete;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<PairComplete>), ReadlineError> {
        let mut path = PathBuf::from(self.env.get_cwd());
        if std::env::set_current_dir(&path).is_ok() {
            self.completer.complete(line, pos, ctx)
        } else {
            let mut segment = String::new();

            if !line.is_empty() {
                for (i, ch) in line.chars().enumerate() {
                    if ch.is_whitespace()
                        || ch == ';'
                        || ch == '\''
                        || ch == '('
                        || ch == ')'
                        || ch == '{'
                        || ch == '}'
                        || ch == '"'
                    {
                        segment = String::new();
                    } else {
                        segment.push(ch);
                    }

                    if i == pos {
                        break;
                    }
                }

                if !segment.is_empty() {
                    path.push(segment.clone());
                }
            }

            let path_str = (path.into_os_string().into_string().unwrap()
                + if segment.is_empty() { "/" } else { "" })
            .replace("/./", "/")
            .replace("//", "/");
            let (pos, mut pairs) =
                self.completer
                    .complete(path_str.as_str(), path_str.len(), ctx)?;
            for pair in &mut pairs {
                pair.replacement = String::from(line) + &pair.replacement.replace(&path_str, "");
            }
            Ok((pos, pairs))
        }
    }
}

/** 语法高亮处理 */
fn syntax_highlight(line: &str) -> String {
    let (tokens, diagnostics) = dune::tokenize(line);

    let mut result = String::new();
    let mut is_colored = false;

    for (token, diagnostic) in tokens.iter().zip(&diagnostics) {
        match (token.kind, token.range.to_str(line)) {
            (TokenKind::BooleanLiteral, b) => {
                result.push_str("\x1b[95m");
                is_colored = true;
                result.push_str(b);
            }
            (
                TokenKind::Punctuation,
                o @ ("@" | "\'" | "=" | "|" | ">>" | "<<" | ">>>" | "->" | "~>"),
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

/** Hinter实现，提供代码补全 */
impl Hinter for DuneHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        let mut segment = String::new();

        if !line.is_empty() {
            for (i, ch) in line.chars().enumerate() {
                if ch.is_whitespace()
                    || ch == ';'
                    || ch == '\''
                    || ch == '('
                    || ch == ')'
                    || ch == '{'
                    || ch == '}'
                    || ch == '"'
                {
                    segment = String::new();
                } else {
                    segment.push(ch);
                }

                if i == pos {
                    break;
                }
            }
        }

        let cmds = vec![
            "exit 0", "ls ", "rm -ri ", "cp -r ", "head ", "tail ", "cd ", "clear",
        ];
        if line.trim().is_empty() {
            return self.hinter.hint(line, pos, ctx);
        } else {
            for cmd in &cmds {
                if cmd.contains(line) {
                    return Some(cmd.trim_start_matches(line).to_string());
                }
            }
        }
        self.hinter.hint(line, pos, ctx)
    }
}

/** Highlighter实现，提供语法高亮 */
impl Highlighter for DuneHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        _prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Borrowed(&self.colored_prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Owned(syntax_highlight(line))
    }

    fn highlight_char(&self, line: &str, _pos: usize) -> bool {
        syntax_highlight(line) != line
    }
}

/** Validator实现，提供输入验证 */
impl Validator for DuneHelper {
    fn validate(&self, _: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

fn get_os_name(t: &Type) -> String {
    match t {
        Type::Alpine => "alpine",
        Type::Amazon => "amazon",
        Type::Android => "android",
        Type::Arch => "arch",
        Type::CentOS => "centos",
        Type::Debian => "debian",
        Type::Macos => "macos",
        Type::Fedora => "fedora",
        Type::Linux => "linux",
        Type::Manjaro => "manjaro",
        Type::Mint => "mint",
        Type::openSUSE => "opensuse",
        Type::EndeavourOS => "endeavouros",
        Type::OracleLinux => "oraclelinux",
        Type::Pop => "pop",
        Type::Redhat => "redhat",
        Type::RedHatEnterprise => "redhatenterprise",
        Type::Redox => "redox",
        Type::Solus => "solus",
        Type::SUSE => "suse",
        Type::Ubuntu => "ubuntu",
        Type::Windows => "windows",
        Type::Unknown | _ => "unknown",
    }
    .to_string()
}

fn get_os_family(t: &Type) -> String {
    match t {
        Type::Amazon | Type::Android => "android",
        Type::Alpine
        | Type::Arch
        | Type::CentOS
        | Type::Debian
        | Type::Fedora
        | Type::Linux
        | Type::Manjaro
        | Type::Mint
        | Type::openSUSE
        | Type::EndeavourOS
        | Type::OracleLinux
        | Type::Pop
        | Type::Redhat
        | Type::RedHatEnterprise
        | Type::SUSE
        | Type::Ubuntu => "linux",

        Type::Macos | Type::Solus | Type::Redox => "unix",

        Type::Windows => "windows",

        Type::Unknown | _ => "unknown",
    }
    .to_string()
}

/// 解析脚本
fn parse(input: &str) -> Result<Expression, Error> {
    match parse_script(input) {
        Ok(result) => Ok(result),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            Err(Error::SyntaxError(input.into(), e))
        }
        Err(nom::Err::Incomplete(_)) => {
            Err(Error::SyntaxError(input.into(), SyntaxError::InternalError))
        }
    }
}

/// 启动交互式REPL（读取-评估-打印循环）
fn repl(
    atomic_rl: Arc<Mutex<Editor<DuneHelper>>>, // 用于线程安全的REPL编辑器
    atomic_env: Arc<Mutex<Environment>>,       // 用于线程安全的环境
) -> Result<(), Error> {
    let mut lines = vec![]; // 用于存储输入的多行代码

    let history_path = get_history_path(); // 获取历史记录文件路径
    loop {
        let mut env = atomic_env.lock().unwrap(); // 锁定环境并获取可变引用
        let mut rl = atomic_rl.lock().unwrap(); // 锁定REPL编辑器并获取可变引用
        let cwd = env.get_cwd(); // 获取当前工作目录
                                 // 设置提示符，根据是否有输入行来决定
                                 // let prompt = format!("{}", Expression::Apply(Box::new(env.get("prompt").unwrap()), vec![env.get_cwd().into()]).eval(&mut env)?);

        let prompt = Expression::Apply(
            Box::new(Expression::Symbol(
                if lines.is_empty() {
                    "prompt"
                } else {
                    "incomplete_prompt"
                }
                .to_string(),
            )),
            vec![cwd.clone().into()], // 传递当前工作目录
        )
        .eval(&mut env) // 计算提示符表达式
        .unwrap_or_else(|_| format!("{}$ ", cwd).into()) // 默认提示符格式
        .to_string();
        rl.helper_mut()
            .expect("No helper")
            .set_prompt(prompt.clone()); // 设置REPL的提示符
        rl.helper_mut().expect("No helper").update_env(&env); // 更新REPL的环境
        let line = readline(prompt, &mut rl); // 读取用户输入
        lines.push(line.clone()); // 将输入行添加到lines中
        let text = lines.join("\n"); // 将多行代码连接成一行

        // 解析输入文本
        match parse(&text) {
            Ok(expr) => {
                // 将当前输入添加到历史记录
                rl.add_history_entry(text.as_str());
                if let Some(path) = &history_path {
                    // 保存历史记录到文件
                    if rl.save_history(path).is_err() {
                        eprintln!("Failed to save history");
                    }
                }
                let val = expr.eval(&mut env); // 评估表达式
                match val.clone() {
                    Ok(Expression::Symbol(name)) => {
                        if let Err(e) =
                            Expression::Apply(Box::new(Expression::Symbol(name)), vec![])
                                .eval(&mut env)
                        // 调用符号对应的函数
                        {
                            eprintln!("{}", e) // 打印错误
                        }
                    }
                    Ok(Expression::None) => {} // 如果评估结果为None，则不做任何处理
                    Ok(Expression::Macro(_, _)) => {
                        let _ = Expression::Apply(
                            Box::new(Expression::Symbol("report".to_string())), // 使用"report"符号
                            vec![Expression::Apply(
                                Box::new(val.unwrap().clone()), // 传递评估结果
                                vec![env.get_cwd().into()],     // 传递当前工作目录
                            )],
                        )
                        .eval(&mut env); // 调用"report"符号对应的函数
                    }
                    Ok(val) => {
                        let _ = Expression::Apply(
                            Box::new(Expression::Symbol("report".to_string())),
                            vec![Expression::Quote(Box::new(val))],
                        )
                        .eval(&mut env);
                    }
                    Err(e) => {
                        eprintln!("{}", e) // 打印错误
                    }
                }
                lines = vec![];
            }

            Err(e) => {
                if line.is_empty() {
                    eprintln!("{}", e); // 打印解析错误
                    lines = vec![];
                } else {
                    rl.add_history_entry(text.as_str());
                }
            }
        }
    }
}

fn run_text(text: &str, env: &mut Environment) -> Result<Expression, Error> {
    parse(text)?.eval(env)
}

fn run_file(path: PathBuf, env: &mut Environment) -> Result<Expression, Error> {
    match std::fs::read_to_string(path) {
        Ok(prelude) => run_text(&prelude, env),
        Err(e) => Err(Error::CustomError(format!("Failed to read file: {}", e))),
    }
}
/// 启动主函数
fn main() -> Result<(), Error> {
    let matches = App::new(
        r#"
        888
        888
        888
    .d88888 888  888 88888b.   .d88b.
   d88" 888 888  888 888 "88b d8P  Y8b
   888  888 888  888 888  888 88888888
   Y88b 888 Y88b 888 888  888 Y8b.
    "Y88888  "Y88888 888  888  "Y8888
   "#,
    )
    .author(crate_authors!())
    .about(crate_description!())
    .args(&[
        arg!([FILE] "Execute a given input file"),
        arg!(-i --interactive "Start an interactive REPL"),
        arg!(-x --exec <INPUT> ... "Execute a given input string")
            .multiple_values(true)
            .required(false),
    ])
    .get_matches(); // 获取命令行参数
    let mut env = Environment::new(); // 创建环境

    binary::init(&mut env); // 初始化二进制文件

    // 解析并执行"clear"表达式
    parse("let clear = _ ~> console@clear ()")?.eval(&mut env)?;
    parse("let pwd = _ ~> echo CWD")?.eval(&mut env)?;
    parse(
        "let join = sep -> l -> {
            let sep = str sep;
            fn@reduce (x -> y -> x + sep + (str y)) (str l@0) (list@tail l)
        }",
    )?
    .eval(&mut env)?;

    parse(
        "let prompt = cwd -> \
            fmt@bold ((fmt@dark@blue \"(dune) \") + \
            (fmt@bold (fmt@dark@green cwd)) + \
            (fmt@bold (fmt@dark@blue \"$ \")))",
    )?
    .eval(&mut env)?;
    parse(
        r#"let incomplete_prompt = cwd ->
            ((len cwd) + (len "(dune) ")) * " " + (fmt@bold (fmt@dark@yellow "> "));"#,
    )?
    .eval(&mut env)?;

    if matches.is_present("FILE") {
        // 获取文件路径
        let path = PathBuf::from(matches.value_of("FILE").unwrap());

        if let Err(e) = run_file(path, &mut env) {
            // 运行文件
            // 打印错误
            eprintln!("{}", e)
        }

        if !matches.is_present("interactive") && !matches.is_present("exec") {
            return Ok(());
        }
    }

    if matches.is_present("exec") {
        match run_text(
            &matches
                .values_of("exec")
                .unwrap()
                .map(String::from)
                .collect::<Vec<_>>()
                .join(" "),
            &mut env,
        ) {
            Ok(result) => {
                Expression::Apply(
                    Box::new(Expression::Symbol("report".to_string())),
                    vec![result],
                )
                .eval(&mut env)?;
            }
            Err(e) => eprintln!("{}", e),
        }

        if !matches.is_present("interactive") {
            return Ok(()); // 如果没有交互模式和执行模式，则退出
        }
    }

    if let Some(home_dir) = dirs::home_dir() {
        // 获取预lude文件路径
        let prelude_path = home_dir.join(".dune-prelude");
        // 如果文件不存在
        if !prelude_path.exists() {
            let prompt = format!("Could not find prelude file at: {}\nWould you like me to write the default prelude to this location? (y/n)\n>>> ", prelude_path.display());
            let mut rl = new_editor(&env); // 创建新的REPL编辑器
            let response = readline(prompt, &mut rl); // 读取用户输入

            if response.to_lowercase().trim() == "y" {
                if let Err(e) = std::fs::write(&prelude_path, DEFAULT_PRELUDE) {
                    eprintln!("Error while writing prelude: {}", e);
                }
            }

            if let Err(e) = run_text(INTRO_PRELUDE, &mut env) {
                eprintln!("Error while running introduction prelude: {}", e);
            }
        } else if let Err(e) = run_file(prelude_path, &mut env) {
            let prompt = format!("Error while running custom prelude: {e}\nWould you like me to write the default prelude to this location? (y/n)\n>>> ");
            let mut rl = new_editor(&env);
            let response = readline(prompt, &mut rl);

            if response.to_lowercase().trim() == "y" {
                if let Err(e) = run_text(INTRO_PRELUDE, &mut env) {
                    eprintln!("Error while running introduction prelude: {}", e);
                }
            }
        }
    }

    // 创建新的REPL编辑器
    let mut rl = new_editor(&env);
    let history_path = get_history_path(); // 获取历史记录文件路径
    if let Some(path) = history_path {
        if rl.load_history(&path).is_err() {} // 加载历史记录，如果失败则不处理
    }

    let editor_ref = Arc::new(Mutex::new(rl)); // 将REPL编辑器包装成线程安全的引用
    let editor_ref_copy = editor_ref.clone(); // 复制REPL编辑器引用

    let env_ref = Arc::new(Mutex::new(env)); // 将环境包装成线程安全的引用
    let env_ref_copy = env_ref.clone(); // 复制环境引用

    ctrlc::set_handler(move || {
        // 设置Ctrl-C处理程序
        repl(editor_ref_copy.clone(), env_ref_copy.clone()).expect("Error in REPL");
    })
    .expect("Error setting Ctrl-C handler");
    // 启动REPL
    repl(editor_ref, env_ref)?;

    Ok(())
}
