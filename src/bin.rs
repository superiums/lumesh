#![allow(clippy::wildcard_in_or_patterns)]

mod binary;

use lumesh::{parse_script, Diagnostic, Environment, Error, Expression, SyntaxError, TokenKind};

use clap::Parser;

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

use std::{
    borrow::Cow::{self, Borrowed, Owned},
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex},
};

#[rustfmt::skip]
const INTRO_PRELUDE: &str = include_str!(".intro-lumesh-prelude");
#[rustfmt::skip]
const DEFAULT_PRELUDE: &str = include_str!(".default-lumesh-prelude");

/// Get the path to the stored history of lumesh commands.
fn get_history_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".lumesh-history"))
}

fn new_editor(env: &Environment) -> Editor<LumeshHelper> {
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
    let h = LumeshHelper {
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

fn readline(prompt: impl ToString, rl: &mut Editor<LumeshHelper>) -> String {
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
struct LumeshHelper {
    completer: FilenameCompleter,
    hinter: HistoryHinter,
    colored_prompt: String,
    validator: MatchingBracketValidator,
    env: Environment,
}

impl LumeshHelper {
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

impl Completer for LumeshHelper {
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

fn syntax_highlight(line: &str) -> String {
    let (tokens, diagnostics) = lumesh::tokenize(line);
    // dbg!(tokens);

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
            (TokenKind::StringRaw, s) => {
                result.push_str("\x1b[38;5;203m");
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

impl Hinter for LumeshHelper {
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

impl Highlighter for LumeshHelper {
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

impl Validator for LumeshHelper {
    fn validate(&self, _: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

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

fn repl(
    atomic_rl: Arc<Mutex<Editor<LumeshHelper>>>,
    atomic_env: Arc<Mutex<Environment>>,
) -> Result<(), Error> {
    let mut lines = vec![];

    let history_path = get_history_path();
    loop {
        let mut env = atomic_env.lock().unwrap();
        let mut rl = atomic_rl.lock().unwrap();
        let cwd = env.get_cwd();
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
            vec![cwd.clone().into()],
        )
        .eval(&mut env)
        .unwrap_or_else(|_| format!("{}$ ", cwd).into())
        .to_string();
        rl.helper_mut()
            .expect("No helper")
            .set_prompt(prompt.clone());
        rl.helper_mut().expect("No helper").update_env(&env);
        let line = readline(prompt, &mut rl);
        lines.push(line.clone());
        let text = lines.join("\n");

        match parse(&text) {
            Ok(expr) => {
                rl.add_history_entry(text.as_str());
                if let Some(path) = &history_path {
                    if rl.save_history(path).is_err() {
                        eprintln!("Failed to save history");
                    }
                }
                let val = expr.eval(&mut env);
                match val.clone() {
                    Ok(Expression::Symbol(name)) => {
                        if let Err(e) =
                            Expression::Apply(Box::new(Expression::Symbol(name)), vec![])
                                .eval(&mut env)
                        {
                            eprintln!("{}", e)
                        }
                    }
                    Ok(Expression::None) => {}
                    Ok(Expression::Macro(_, _)) => {
                        let _ = Expression::Apply(
                            Box::new(Expression::Symbol("report".to_string())),
                            vec![Expression::Apply(
                                Box::new(val.unwrap().clone()),
                                vec![env.get_cwd().into()],
                            )],
                        )
                        .eval(&mut env);
                    }
                    Ok(val) => {
                        let _ = Expression::Apply(
                            Box::new(Expression::Symbol("report".to_string())),
                            vec![Expression::Quote(Box::new(val))],
                        )
                        .eval(&mut env);
                    }
                    Err(e) => {
                        eprintln!("{}", e)
                    }
                }
                lines = vec![];
            }

            Err(e) => {
                if line.is_empty() {
                    eprintln!("{}", e);
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
fn init_cmds(env: &mut Environment) -> Result<(), Error> {
    parse("let clear = _ ~> console@clear ()")?.eval(env)?;
    parse("let pwd = _ ~> echo CWD")?.eval(env)?;
    parse(
        "let join = sep -> l -> {
                let sep = str sep;
                fn@reduce (x -> y -> x + sep + (str y)) (str l@0) (list@tail l)
            }",
    )?
    .eval(env)?;

    parse(
        "let prompt = cwd -> \
                fmt@bold ((fmt@dark@blue \"(lumesh) \") + \
                (fmt@bold (fmt@dark@green cwd)) + \
                (fmt@bold (fmt@dark@blue \"$ \")))",
    )?
    .eval(env)?;
    parse(
        r#"let incomplete_prompt = cwd ->
                ((len cwd) + (len "(lumesh) ")) * " " + (fmt@bold (fmt@dark@yellow "> "));"#,
    )?
    .eval(env)?;
    Ok(())
}
fn init_config(env: &mut Environment) -> Result<(), Error> {
    if let Some(home_dir) = dirs::home_dir() {
        let prelude_path = home_dir.join(".lumesh-prelude");
        // If file doesn't exist
        if !prelude_path.exists() {
            let prompt = format!("Could not find prelude file at: {}\nWould you like me to write the default prelude to this location? (y/n)\n>>> ", prelude_path.display());
            let mut rl = new_editor(&env);
            let response = readline(prompt, &mut rl);

            if response.to_lowercase().trim() == "y" {
                if let Err(e) = std::fs::write(&prelude_path, DEFAULT_PRELUDE) {
                    eprintln!("Error while writing prelude: {}", e);
                }
            }

            if let Err(e) = run_text(INTRO_PRELUDE, env) {
                eprintln!("Error while running introduction prelude: {}", e);
            }
        } else if let Err(e) = run_file(prelude_path, env) {
            let prompt = format!("Error while running custom prelude: {e}\nWould you like me to write the default prelude to this location? (y/n)\n>>> ");
            let mut rl = new_editor(&env);
            let response = readline(prompt, &mut rl);

            if response.to_lowercase().trim() == "y" {
                if let Err(e) = run_text(INTRO_PRELUDE, env) {
                    eprintln!("Error while running introduction prelude: {}", e);
                }
            }
        }
    }
    Ok(())
}
fn run_repl(env: Environment) -> Result<(), Error> {
    let mut rl = new_editor(&env);
    let history_path = get_history_path();
    if let Some(path) = history_path {
        if rl.load_history(&path).is_err() {}
    }

    let editor_ref = Arc::new(Mutex::new(rl));
    let editor_ref_copy = editor_ref.clone();

    let env_ref = Arc::new(Mutex::new(env));
    let env_ref_copy = env_ref.clone();

    ctrlc::set_handler(move || {
        repl(editor_ref_copy.clone(), env_ref_copy.clone()).expect("Error in REPL");
    })
    .expect("Error setting Ctrl-C handler");

    repl(editor_ref, env_ref)
}

#[derive(Parser)]
#[command(
    name = "lumesh",
    version = env!("CARGO_PKG_VERSION"),
    // author = crate_authors!(),
    // about = crate_description!(),
    // disable_help_flag = true,  // 禁用默认的 --help
    // disable_version_flag = true // 禁用默认的 --version
)]
struct Cli {
    /// 执行字符串命令
    #[arg(short = 'i', long, num_args = 0..1)]
    interactive: bool,

    #[arg(short = 'c', long, num_args = 1..)]
    cmd: Option<Vec<String>>,

    /// 严格模式
    #[arg(short = 's', long)]
    strict: bool,

    /// 脚本文件路径
    #[arg(required = false, num_args = 1, index = 1)]
    file: Option<String>,

    /// 传递给脚本的参数
    #[arg(
        last = true,
        num_args=0..,
        allow_hyphen_values = true,
        requires ="file",
        index = 2
    )]
    argv: Vec<String>,
    // 显示帮助信息
    // #[arg(long, action = clap::ArgAction::Help)]
    // help: Option<bool>,

    // /// 显示版本信息
    // #[arg(short = 'V', long)]
    // version: bool,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    let mut env = Environment::new();
    env.define(
        "argv",
        Expression::List(cli.argv.into_iter().map(Expression::String).collect()),
    );
    binary::init(&mut env);

    // 处理命令行参数优先级
    // if cli.version {
    //     println!("lumesh {}", env!("CARGO_PKG_VERSION"));
    //     return;
    // }

    if let Some(cmd_parts) = cli.cmd {
        // -c 模式
        let cmd = cmd_parts.join(" ");
        match run_text(cmd.as_str(), &mut env) {
            Ok(result) => {
                Expression::Apply(
                    Box::new(Expression::Symbol("report".to_string())),
                    vec![result],
                )
                .eval(&mut env)?;
            }
            Err(e) => eprintln!("{}", e),
        }
    } else if let Some(file) = cli.file {
        // 文件执行模式
        let path = PathBuf::from(file);
        if let Err(e) = run_file(path, &mut env) {
            eprintln!("{}", e)
        }
        if cli.interactive {
            init_cmds(&mut env)?;
            init_config(&mut env)?;
            run_repl(env)?;
        }
    } else {
        // 自动进入交互模式
        init_cmds(&mut env)?;
        // TODO: split config for interactive or global.
        init_config(&mut env)?;
        run_repl(env)?;
    }
    Ok(())
}
