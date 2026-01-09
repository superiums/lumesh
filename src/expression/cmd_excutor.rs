use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind, childman,
    expression::pty::exec_in_pty,
    runtime::{IFS_CMD, ifs_contains},
    utils::expand_home,
};

use super::eval::State;

use glob::glob;
// use portable_pty::ChildKiller;
// use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::{
    io::Write,
    process::{Command, Stdio},
};

/// mode: 1=null_stdout, 2=null_err, 4=err_to_stdout,
/// 8=background, 11=background,shutdown_all
/// 16=pty
/// 执行单个命令（支持管道）
fn exec_single_cmd(
    job: &Expression,
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>, // 前一条命令的输出（None 表示第一个命令）
    pipe_out: bool,
    mode: u8,
    depth: usize,
) -> Result<Option<Vec<u8>>, RuntimeError> {
    // dbg!("------ exec:------", &cmdstr, &args);
    // dbg!(&mode, &pipe_out, &input.is_some());
    // dbg!(&input);
    if mode & 16 != 0 {
        // spawn_in_pty(cmdstr, args, env, input);
        return exec_in_pty(cmdstr, args, env, input)
            .map_err(|e| RuntimeError::new(e, job.clone(), depth));
    }
    let mut cmd = Command::new(cmdstr);

    let ar = args.unwrap_or_default();

    cmd.args(ar)
        .envs(env.get_root().get_bindings_string())
        .current_dir(
            std::env::current_dir().map_err(|e| {
                RuntimeError::from_io_error(e, "get cwd".into(), job.clone(), depth)
            })?,
        );

    // 设置 stdin
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::inherit());
    }

    // 设置 stdout（如果是交互式命令，直接接管终端）
    if pipe_out {
        cmd.stdout(Stdio::piped());
    } else if mode & 1 != 0 {
        cmd.stdout(Stdio::null());
    } else {
        // if mode == 0 {
        cmd.stdout(Stdio::inherit());
    }

    // 设置 stderr
    if mode & 2 != 0 {
        cmd.stderr(Stdio::null());
    } else if mode & 4 != 0 {
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stderr(Stdio::inherit());
    }

    // 执行命令
    let mut child = cmd.spawn().map_err(|e| match &e.kind() {
        std::io::ErrorKind::NotFound => RuntimeError::new(
            RuntimeErrorKind::ProgramNotFound(cmdstr.clone()),
            job.clone(),
            depth,
        ),
        std::io::ErrorKind::PermissionDenied => RuntimeError::new(
            RuntimeErrorKind::PermissionDenied(cmdstr.clone()),
            job.clone(),
            depth,
        ),
        _ => RuntimeError::from_io_error(
            e,
            format!("spawn cmd `{cmdstr}`").into(),
            job.clone(),
            depth,
        ),
    })?;

    // 写入输入
    if let Some(input) = input {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&input)
            .map_err(|e| {
                RuntimeError::from_io_error(
                    e,
                    format!("pipe stdin to `{cmdstr}`").into(),
                    job.clone(),
                    depth,
                )
            })?;
    }

    // TODO not work yet
    // 合并 stderr 和 stdout 的流
    if mode & 4 != 0 {
        if let Some(mut stderr) = child.stderr.take() {
            // if let Some(mut stdout) = child.stdout.take() {
            std::io::copy(&mut stderr, &mut std::io::stdout()).unwrap(); // 将 stderr 合并到 stdout
            // }
        }
    }

    // 中断信号处理
    // let mut child_ref = child;
    childman::set_child(child.id());
    // std::thread::spawn(move || {
    //     loop {
    //         if state::read_signal() {
    //             let _ = child_ref.kill();
    //             break;
    //         }
    //         std::thread::sleep(Duration::from_secs(1));
    //     }
    // });

    // 获取输出
    if pipe_out {
        // 管道捕获
        let output = child.wait_with_output().map_err(|e| {
            RuntimeError::from_io_error(
                e,
                format!("wait output of cmd `{cmdstr}`").into(),
                job.clone(),
                depth,
            )
        })?;
        childman::clear_child();

        if output.status.success() {
            if mode & 1 == 0 {
                //未关闭标准输出才返回结果
                Ok(Some(output.stdout))
            } else {
                Ok(None)
            }
        } else if mode & 4 != 0 {
            //错误输出>标准输出
            let mut combined = Vec::new();
            combined.extend(output.stdout);
            // println!(
            //     "err output: {}",
            //     String::from_utf8_lossy(&output.stderr.clone().as_ref())
            // );
            combined.extend(output.stderr);
            // println!("Combined output: {}", String::from_utf8_lossy(&combined));
            return Ok(Some(combined));
        } else {
            if mode & 2 == 0 {
                //未关闭错误输出才返回错误
                let stderr = String::from_utf8_lossy(output.stderr.as_ref())
                    .trim()
                    .to_string();
                if stderr.is_empty() {
                    // return Err(RuntimeError::CommandFailed2(cmdstr.to_owned(), stderr));
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::CommandFailed2(cmdstr.to_owned(), stderr),
                        job.clone(),
                        depth,
                    ));
                }
            } else if mode & 1 == 0 {
                // 如果关闭了错误输出，则尝试返回标准输出，二者可能同时存在。
                return Ok(Some(output.stdout));
            }
            return Ok(None);
        }
    } else if mode & 8 != 0 {
        // 后台运行
        return Ok(None);
    } else {
        // 正常模式
        let status = child.wait().map_err(|e| {
            RuntimeError::from_io_error(
                e,
                format!("wait cmd `{cmdstr}`").into(),
                job.clone(),
                depth,
            )
        })?;
        childman::clear_child();

        if status.success() {
            return Ok(None);
        } else if mode & 2 == 0 {
            //未关闭错误输出才返回错误
            Err(RuntimeError::new(
                RuntimeErrorKind::CommandFailed2(cmdstr.to_owned(), status.to_string()),
                job.clone(),
                depth,
            ))
        } else {
            Ok(None)
        }
    }
}

// 管道
pub fn handle_command(
    job: &Expression,
    cmd: &String,
    args: &Vec<Expression>,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, RuntimeError> {
    // dbg!("   3.--->handle_command:", &cmd, &args);

    let is_in_assign = state.contains(State::IN_ASSIGN);
    let pipe_out = is_in_assign || state.contains(State::IN_PIPE);
    let mut cmd_args = vec![];
    state.set(State::SKIP_BUILTIN_SEEK | State::IN_ASSIGN);

    for arg in args {
        // for flattened_arg in Expression::flatten(vec![arg.eval_mut(env, depth + 1)?]) {
        // dbg!("     4.--->arg:", &arg, arg.type_name());
        let e_arg = arg.eval_mut(state, env, depth + 1)?;
        // dbg!("     4.--->evaluated_arg:", &e_arg, e_arg.type_name());

        match e_arg {
            Expression::Symbol(s) => cmd_args.push(s),
            Expression::String(st) => {
                let s = expand_home(&st).to_string();
                if s.contains('*') {
                    let mut matched = false;
                    if let Some(g) = glob(&s).ok() {
                        for path in g.filter_map(Result::ok) {
                            matched = true;
                            cmd_args.push(path.to_string_lossy().to_string());
                        }
                    }
                    if !matched {
                        return Err(RuntimeError {
                            kind: RuntimeErrorKind::WildcardNotMatched(s.to_string()),
                            context: job.clone(),
                            depth,
                        });
                        // cmd_args.push(s);
                    }
                } else {
                    // 分割多参数字符串
                    if ifs_contains(IFS_CMD, env) {
                        let ifs = env.get("IFS");
                        let sp = match &ifs {
                            Some(Expression::String(fs)) => s.split_terminator(fs.as_str()),
                            _ => s.split_terminator("\n"),
                        };
                        sp.for_each(|v| cmd_args.push(v.to_string()));
                    } else {
                        cmd_args.push(s.to_string())
                    }
                }
            }
            Expression::List(ls) => {
                ls.iter().for_each(|a| cmd_args.push(format!("{a}")));
            }
            Expression::Bytes(b) => cmd_args.push(String::from_utf8_lossy(&b).to_string()),
            Expression::None => continue,
            _ => cmd_args.push(format!("{e_arg}")),
        }
    }
    state.clear(State::SKIP_BUILTIN_SEEK);
    if !is_in_assign {
        state.clear(State::IN_ASSIGN);
    }

    #[cfg(unix)]
    let pty_cmds = [
        "lume", "bash", "sh", "fish", "top", "btop", "vi", "passwd", "ssh", "script", "expect",
        "telnet", "screen", "tmux", "ftp",
    ];
    #[cfg(windows)]
    let pty_cmds = [
        "lume",
        "fish",
        "ssh",
        "telnet",
        "screen",
        "tmux",
        "cmd.exe",
        "PowerShell",
        "Cygwin",
        "WinPTY",
        "ConPTY",
    ];
    let cmd_mode: u8 = match state.contains(State::PTY_MODE) || pty_cmds.contains(&cmd.as_str()) {
        true => 16,
        false => match cmd_args.last() {
            Some(s) => match s.as_str() {
                "&" => {
                    cmd_args.pop();
                    11
                }
                "&-" => {
                    cmd_args.pop();
                    1
                }
                "&?" => {
                    cmd_args.pop();
                    2
                }
                "&." => {
                    cmd_args.pop();
                    3
                }
                "&+" => {
                    cmd_args.pop();
                    4
                }
                _ => 0,
            },
            _ => 0,
        },
    };
    // dbg!(args, &cmd_args);
    let last_input = state.pipe_out();
    let pipe_input = to_bytes(last_input);
    let result = exec_single_cmd(
        job,
        cmd,
        Some(cmd_args),
        env,
        pipe_input,
        pipe_out,
        cmd_mode,
        depth,
    )?;
    Ok(to_expr(result))
}

pub fn to_expr(bytes_out: Option<Vec<u8>>) -> Expression {
    match bytes_out {
        Some(b) => Expression::String(String::from_utf8_lossy(&b).trim().to_string()),
        _ => Expression::None,
    }
}
fn to_bytes(expr_out: Option<Expression>) -> Option<Vec<u8>> {
    expr_out.map(|p| p.to_string().as_bytes().to_owned())
}
