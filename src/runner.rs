use lumesh::parse_and_eval;
use lumesh::runtime::{init_config, run_file};
use lumesh::{Environment, Expression};
use std::path::Path;
use std::path::PathBuf;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("script file or command is expected.");
        std::process::exit(0);
    });

    let mut args = std::env::args().skip(1); // 跳过二进制名称
    let mut cmd = None;
    let mut file = None;
    let mut script_args = Vec::new();

    let mut env = Environment::new();
    // is login shell
    let is_login_shell = std::env::args()
        .next()
        .map(|arg| {
            Path::new(&arg)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&arg) // 使用文件名或原始参数
                .starts_with('-')
        })
        .unwrap_or(false);
    env.define("IS_LOGIN", Expression::Boolean(is_login_shell));
    // global env
    if !is_login_shell {
        for (key, value) in std::env::vars() {
            env.define(&mut key.to_owned(), Expression::String(value));
        }
    }

    // 原生参数解析
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" => {
                println!("usage:");
                println!("      lumesh [options] [file] <args...>");
                println!("      lumesh [options] -c [command]");
                println!("      lumesh -h");
                println!("");
                println!("options:");
                // println!("      -s: for strict mode.");
                println!("      -p: profile.");
                println!("      -h: for help.");
                println!("");
                println!("this is a swift script runtime without interactive.");
                println!("for interactive, use lume instead.");
                std::process::exit(0);
            }
            // "-s" => {
            //     // strict mode
            //     unsafe {
            //         STRICT = true;
            //     }
            //     env.define("IS_STRICT", Expression::Boolean(true));
            // }
            "-p" => {
                let profile = args.next().unwrap_or_else(|| {
                    eprintln!("-p needs profile path");
                    std::process::exit(0);
                });
                env.define("LUME_PROFILE", Expression::String(profile));
            }
            "-c" => {
                cmd = Some(args.next().unwrap_or_else(|| {
                    eprintln!("-c needs command.");
                    std::process::exit(0);
                }));
            }
            _ => {
                if file.is_none() {
                    // 第一个非选项参数视为文件路径
                    file = Some(arg);
                    // 剩余参数作为脚本参数
                    script_args = args.collect();
                    break;
                }
            }
        }
    }

    // config
    init_config(&mut env);

    env.define("SCRIPT", Expression::String(path));
    env.define(
        "argv",
        Expression::from(
            script_args
                .into_iter()
                .map(Expression::String)
                .collect::<Vec<Expression>>(),
        ),
    );

    // run
    if let Some(cmd_str) = cmd {
        parse_and_eval(&cmd_str, &mut env);
    } else if let Some(file_path) = file {
        let path = PathBuf::from(file_path);
        run_file(path, &mut env);
    }
}
