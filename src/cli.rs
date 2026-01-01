#![allow(clippy::wildcard_in_or_patterns)]

// mod binary;
use clap::Parser;
use lumesh::CFM_ENABLED;
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

    /// force interactive mode
    #[arg(short = 'i', long)]
    interactive: bool,

    /// NO command first mode
    #[arg(short = 'm', long)]
    cfmoff: bool,

    /// NO ai mode
    #[arg(short = 'a', long)]
    aioff: bool,

    /// NO history (private) mode
    #[arg(short = 'n', long)]
    nohistory: bool,

    /// command to eval
    #[arg(short = 'c', long, num_args = 1)]
    cmd: Option<String>,

    /// script file and args to execute
    #[arg(
        required=false,
        num_args=1..,
        index = 1,
        allow_hyphen_values = true,
    )]
    file_n_args: Option<Vec<String>>,

    /// args for cmd
    #[arg(
        last = true,
        num_args=0..,
        index = 2,
        allow_hyphen_values = true,
    )]
    cmd_argv: Vec<String>,
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
    // std::env::args().for_each(|a| println!("{}", &a));
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

    // println!("file_n_args {:?}", &cli.file_n_args);
    // println!("file {:?}", &cli.file);
    // println!("cmd_argv {:?}", &cli.cmd_argv);

    // profile
    if let Some(profile) = cli.profile {
        cli_env.define("LUME_PROFILE", Expression::String(profile));
    }
    if cli.nohistory {
        cli_env.define("LUME_NO_HISTORY", Expression::Boolean(true));
    }

    // 命令执行模式
    if let Some(cmd) = cli.cmd {
        cli_env.define("IS_INTERACTIVE", Expression::Boolean(cli.interactive));
        let argv = match cli.file_n_args {
            Some(fa) => fa,       //accept 'cmd a b -c --d -- e' which goes to cli.file_n_args
            None => cli.cmd_argv, //accept 'cmd -- a b -c --d e' which goes to cli.cmd_argv
        };
        cli_env.define(
            "argv",
            Expression::from(argv.into_iter().map(Expression::String).collect::<Vec<_>>()),
        );
        env_config(&mut cli_env, cli.aioff, cli.strict);

        parse_and_eval(cmd.as_str(), &mut cli_env);

        if cli.interactive {
            set_cfm(!cli.cfmoff);
            repl::run_repl(&mut cli_env);
        }
    }
    // 文件执行模式
    else if let Some(file_n_args) = cli.file_n_args {
        if let Some((s, args)) = file_n_args.split_first() {
            cli_env.define("IS_INTERACTIVE", Expression::Boolean(false));
            cli_env.define("SCRIPT", Expression::String(s.to_owned()));
            cli_env.define(
                "argv",
                Expression::from(
                    args.iter()
                        .map(|a| Expression::String(a.to_owned()))
                        .collect::<Vec<_>>(),
                ),
            );
            env_config(&mut cli_env, cli.aioff, cli.strict);
            // let path = PathBuf::from(file);
            run_file(s, &mut cli_env);
        }
    }
    // 纯交互模式
    else {
        cli_env.define("IS_INTERACTIVE", Expression::Boolean(true));

        env_config(&mut cli_env, cli.aioff, cli.strict);
        set_cfm(!cli.cfmoff);
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

fn set_cfm(cfm: bool) {
    // cli_env.define("LUME_NO_CFM", Expression::Boolean(cli.cfmoff));
    unsafe {
        CFM_ENABLED = cfm;
    }
}
