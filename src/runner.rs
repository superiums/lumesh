use lumesh::parse_and_eval;
use lumesh::runtime::{init_config, run_file};
use lumesh::{Environment, Expression};
use std::path::{Path, PathBuf};
// use std::path::PathBuf;

fn main() {
    // 获取所有命令行参数
    let args: Vec<String> = std::env::args().collect();

    // 初始化变量
    let mut cmd = None; // 存储 `-c` 参数
    let mut file = None; // 存储脚本文件路径
    let mut script_args = Vec::new(); // 存储脚本参数
    let mut is_command_mode = false; // 是否处于 `-c` 模式
    let mut is_script_mode = false; // 是否处于脚本模式
    let mut env = Environment::new();

    // 判断是否为登录 shell
    let is_login_shell = args
        .first()
        .map(|arg| {
            Path::new(arg)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(arg)
                .starts_with('-')
        })
        .unwrap_or(false);

    // 如果不是登录 shell，加载环境变量
    if !is_login_shell {
        for (key, value) in std::env::vars() {
            env.define(key.as_str(), Expression::String(value));
        }
    }

    // 遍历参数
    for arg in args.iter().skip(1) {
        if is_script_mode {
            // 已进入脚本参数模式
            script_args.push(arg.clone());
        } else if arg == "--" {
            // 遇到 `--`，切换到脚本参数模式
            is_script_mode = true;
        } else if arg == "-c" {
            // 处理 `-c` 参数
            is_command_mode = true;
        } else if is_command_mode {
            // 累积 `-c` 后的命令片段
            cmd.get_or_insert_with(Vec::new).push(arg.clone());
        } else {
            // 处理普通参数
            if file.is_none() {
                // 第一个非选项参数视为文件路径
                file = Some(arg.clone());
            } else {
                // 其他参数视为脚本参数
                script_args.push(arg.clone());
            }
        }
    }

    let mut runner_env = env.fork();
    runner_env.define("IS_LOGIN", Expression::Boolean(is_login_shell));
    // config
    init_config(&mut runner_env);

    runner_env.define(
        "argv",
        Expression::from(
            script_args
                .into_iter()
                .map(Expression::String)
                .collect::<Vec<Expression>>(),
        ),
    );

    // run
    if let Some(cmd_part) = cmd {
        parse_and_eval(&cmd_part.join(" "), &mut runner_env);
    } else if let Some(file_path) = file {
        let pathbf = PathBuf::from(file_path);
        run_file(pathbf, &mut runner_env);
    }
}
