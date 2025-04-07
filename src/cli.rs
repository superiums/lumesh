#![allow(clippy::wildcard_in_or_patterns)]

mod binary;
use clap::Parser;
use lumesh::repl; // 新增模块引用
                  // use lumesh::binary;
use lumesh::runtime::{run_file, run_text};
use lumesh::{Environment, Error, Expression};
use std::path::PathBuf;

// #[rustfmt::skip]
// const INTRO_PRELUDE: &str = include_str!("repl/.intro-lumesh-prelude");

// 移除以下被移动的代码：
// - get_history_path()
// - new_editor()
// - strip_ansi_escapes()
// - readline()
// - LumeshHelper 及其实现
// - syntax_highlight()
// - repl()
// - run_repl()
// - init_config()

#[derive(Parser)]
#[command(
    name = "lumesh",
    version = env!("CARGO_PKG_VERSION"),
    about = "Lumesh scripting language runtime"
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

    // 初始化核心环境
    env.define(
        "argv",
        Expression::List(cli.argv.into_iter().map(Expression::String).collect()),
    );
    binary::init(&mut env);

    // 命令执行模式
    if let Some(cmd_parts) = cli.cmd {
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
    }
    // 文件执行模式
    else if let Some(file) = cli.file {
        let path = PathBuf::from(file);
        if let Err(e) = run_file(path, &mut env) {
            eprintln!("{}", e)
        }
        if cli.interactive {
            // repl::init_cmds(&mut env)?; // 调用 REPL 初始化
            // repl::init_config(&mut env)?;
            repl::run_repl(env)?;
        }
    }
    // 纯交互模式
    else {
        // repl::init_cmds(&mut env)?; // 调用 REPL 初始化
        // repl::init_config(&mut env)?;
        repl::run_repl(env)?;
    }
    Ok(())
}
