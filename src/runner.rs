// src/bin/runner.rs
mod binary;
use lumesh::runtime::{run_file, run_text};
use lumesh::{Environment, Error, Expression};
use std::path::Path;
use std::path::PathBuf;
// 删除原有的 Cli 结构体定义

fn main() -> Result<(), Error> {
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

    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("script file or command is expected.");
        std::process::exit(0);
    });

    let mut args = std::env::args().skip(1); // 跳过二进制名称
    let mut cmd = None;
    let mut file = None;
    let mut script_args = Vec::new();

    // 原生参数解析
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-c" => {
                cmd = Some(args.next().unwrap_or_else(|| {
                    eprintln!("-c needs command.");
                    std::process::exit(0);
                }));
            }
            // "-s" => { /* 严格模式保留但不实现 */ }
            "-h" => {
                println!("usage:");
                println!("      lumesh [file] <args...>");
                println!("      lumesh -c [command]");
                println!("      lumesh -h");
                println!("");
                // println!("options:");
                // println!("      -s: for strict mode.");
                // println!("      -h: for help.");
                println!("");
                println!("this is a swift script runtime without interactive.");
                println!("for interactive, use lume instead.");
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

    let mut env = Environment::new();
    env.define("LOGIN_SHELL", Expression::Boolean(is_login_shell));
    env.define("SCRIPT", Expression::String(path));
    env.define(
        "argv",
        Expression::List(script_args.into_iter().map(Expression::String).collect()),
    );
    binary::init(&mut env);

    // 执行逻辑保持不变
    if let Some(cmd_str) = cmd {
        match run_text(&cmd_str, &mut env) {
            Ok(result) => {
                Expression::Apply(
                    Box::new(Expression::Symbol("report".to_string())),
                    vec![result],
                )
                .eval(&mut env)?;
            }
            Err(e) => eprintln!("{}", e),
        }
    } else if let Some(file_path) = file {
        let path = PathBuf::from(file_path);
        if let Err(e) = run_file(path, &mut env) {
            eprintln!("{}", e)
        }
    }
    Ok(())
}
