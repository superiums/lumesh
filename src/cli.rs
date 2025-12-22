#![allow(clippy::wildcard_in_or_patterns)]

// mod binary;
use clap::Parser;
use lumesh::parse_and_eval;
use lumesh::repl;
use lumesh::runtime::init_config;
// 新增模块引用
// use lumesh::binary;
// use lumesh::ENV;
// use lumesh::STRICT;
use lumesh::runtime::run_file;
use lumesh::{Environment, Expression};
use std::env;
use std::path::Path;

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
    /// config file
    #[arg(short = 'p', long, num_args = 0..1)]
    profile: Option<String>,

    /// strict mode
    #[arg(short = 's', long)]
    strict: bool,

    /// private mode
    #[arg(short = 'n', long)]
    nohistory: bool,

    /// no-ai mode
    #[arg(short = 'a', long)]
    aioff: bool,

    /// force interactive mode
    #[arg(short = 'i', long, num_args = 0..1)]
    interactive: bool,

    /// command to execute
    #[arg(short = 'c', long, num_args = 1..)]
    cmd: Option<Vec<String>>,

    /// script to load
    #[arg(
        required = false,
        num_args = 1,
        index = 1,
        conflicts_with = "interactive"
    )]
    file: Option<String>,

    /// args for script/cmd
    #[arg(
        last = true,
        num_args=0..,
        allow_hyphen_values = true,
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

fn main() {
    let cli = Cli::parse();
    // 初始化核心环境
    let mut env = Environment::new();
    // login
    let is_login_shell = std::env::args()
        .next()
        .map(|arg| {
            Path::new(&arg)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&arg)
                .starts_with('-')
        })
        .unwrap_or(false);
    // global env
    if !is_login_shell {
        for (key, value) in std::env::vars() {
            env.define(&key, Expression::String(value));
        }
    }

    let mut cli_env = env.fork();
    cli_env.define("IS_LOGIN", Expression::Boolean(is_login_shell));
    // argv
    cli_env.define(
        "argv",
        Expression::from(
            cli.argv
                .into_iter()
                .map(Expression::String)
                .collect::<Vec<Expression>>(),
        ),
    );
    // bultiin
    // binary::init(&mut env);

    // profile
    if let Some(profile) = cli.profile {
        cli_env.define("LUME_PROFILE", Expression::String(profile));
    }
    if cli.nohistory {
        cli_env.define("LUME_NO_HISTORY", Expression::Boolean(true));
    }

    // 命令执行模式
    if let Some(cmd_parts) = cli.cmd {
        cli_env.define("IS_INTERACTIVE", Expression::Boolean(cli.interactive));
        env_config(&mut cli_env, cli.aioff, cli.strict);

        let cmd = cmd_parts.join(" ");
        parse_and_eval(cmd.as_str(), &mut cli_env);

        if cli.interactive {
            repl::run_repl(&mut cli_env);
        }
    }
    // 文件执行模式
    else if let Some(file) = cli.file {
        cli_env.define("IS_INTERACTIVE", Expression::Boolean(false));
        cli_env.define("SCRIPT", Expression::String(file.to_owned()));

        env_config(&mut cli_env, cli.aioff, cli.strict);
        // let path = PathBuf::from(file);
        run_file(&file, &mut cli_env);
    }
    // 纯交互模式
    else {
        cli_env.define("IS_INTERACTIVE", Expression::Boolean(true));

        env_config(&mut cli_env, cli.aioff, cli.strict);
        repl::run_repl(&mut cli_env);
    }
}

fn env_config(env: &mut Environment, aioff: bool, strict: bool) {
    init_config(env);

    // strict
    env.define("STRICT", Expression::Boolean(strict));
    // unsafe {
    //     STRICT = strict;
    // }

    // ai off
    if aioff {
        env.undefine("LUME_AI_CONFIG");
    }
}
