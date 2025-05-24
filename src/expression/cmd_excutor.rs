use crate::{Environment, Expression, RuntimeError, get_builtin};

use super::{alias, eval::State};
use glob::glob;
use std::{
    ffi::OsStr,
    io::{ErrorKind, Write},
    process::{Command, Stdio},
    rc::Rc,
};

/// 执行
pub fn eval_command(
    cmd: &Rc<Expression>,
    args: &Rc<Vec<Expression>>,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, RuntimeError> {
    // dbg!("2.--->Command:", &self, &cmd, &args);

    match cmd.as_ref() {
        // index类型的内置命令，或其他保存于map的命令
        Expression::Index(..) => {
            let cmdx = cmd.as_ref().eval_mut(state, env, depth)?;
            // dbg!(&cmd, &cmdx);
            cmdx.apply(args.to_vec()).eval_apply(state, env, depth)
        }
        // 符号
        // string like cmd: ./abc
        // TODO | Expression::String(cmd_sym)
        Expression::Symbol(cmd_sym) => {
            match alias::get_alias(cmd_sym) {
                // 别名
                Some(cmd_alias) => {
                    // dbg!(&cmd_alias.type_name());
                    if !args.is_empty() {
                        match cmd_alias {
                            Expression::Command(cmd_name, cmd_args) => {
                                cmd_args.as_ref().clone().append(&mut args.to_vec());
                                handle_command(&cmd_name.to_string(), &cmd_args, state, env, depth)
                            }
                            Expression::Apply(..) => cmd_alias
                                .append_args(args.to_vec())
                                .eval_mut(state, env, depth),
                            Expression::Index(..) => {
                                let cmdx = cmd_alias.eval_mut(state, env, depth)?;
                                // dbg!(&cmd, &cmdx);
                                cmdx.append_args(args.to_vec())
                                    .eval_apply(state, env, depth)
                            }
                            _ => Err(RuntimeError::TypeError {
                                expected: "Command or Builtin".into(),
                                found: cmd_alias.type_name(),
                            }),
                        }
                    } else {
                        cmd_alias.eval_apply(state, env, depth)
                    }
                }
                _ => {
                    match get_builtin(cmd_sym) {
                        // 顶级内置命令
                        Some(bti) => {
                            // dbg!("branch to builtin:", &cmd, &bti);
                            bti.apply(args.to_vec()).eval_apply(state, env, depth)
                        }
                        // 三方命令
                        _ => handle_command(&cmd_sym.to_string(), args, state, env, depth),
                    }
                }
            }
        }
        e => Err(RuntimeError::TypeError {
            expected: "Symbol".to_string(),
            found: e.type_name(),
        }),
    }
}

/// mode: 1=null_stdout, 2=null_err, 4=err_to_stdout,
/// 8=background, 11=background,shutdown_all
/// 执行单个命令（支持管道）
fn exec_single_cmd<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(
    cmdstr: &String,
    args: Option<I>,
    env: &mut Environment,
    input: Option<Vec<u8>>, // 前一条命令的输出（None 表示第一个命令）
    pipe_out: bool,
    mode: u8,
) -> Result<Option<Vec<u8>>, RuntimeError> {
    // dbg!("------ exec:------", &cmdstr, &args, &is_last);
    // dbg!(&mode);
    let mut cmd = Command::new(cmdstr);
    match args {
        Some(ar) => cmd
            .args(ar)
            .envs(env.get_bindings_string())
            .current_dir(std::env::current_dir()?),
        _ => cmd
            .envs(env.get_bindings_string())
            .current_dir(std::env::current_dir()?),
    };
    cmd.envs(env.get_bindings_string())
        .current_dir(std::env::current_dir()?);
    // 设置 stdin
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::inherit());
    }

    // 设置 stdout（如果是交互式命令，直接接管终端）

    if pipe_out {
        cmd.stdout(Stdio::piped());
    } else {
        if mode & 1 != 0 {
            cmd.stdout(Stdio::null());
        }
        if mode & 2 != 0 {
            cmd.stderr(Stdio::null());
        }
        if mode == 0 || mode & 4 != 0 {
            cmd.stdout(Stdio::inherit());
        }
    }

    // 执行命令
    let mut child = cmd.spawn().map_err(|e| match &e.kind() {
        ErrorKind::NotFound => RuntimeError::ProgramNotFound(cmdstr.into()),
        _ => RuntimeError::CommandFailed2(cmdstr.into(), e.to_string()),
    })?;

    // 写入输入
    if let Some(input) = input {
        child.stdin.as_mut().unwrap().write_all(&input)?;
    }

    // 获取输出

    if pipe_out {
        // 管道捕获
        let output = child.wait_with_output()?;
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

            Ok(Some(combined))
        } else if mode & 2 == 0 {
            //未关闭错误输出才返回错误
            let stderr = String::from_utf8_lossy(output.stderr.as_ref())
                .trim()
                .to_string();
            Err(RuntimeError::CommandFailed2(cmdstr.to_owned(), stderr))
        } else {
            // 如果关闭了错误输出，则尝试返回标准输出，二者可能同时存在。
            Ok(Some(output.stdout))
        }
    } else if mode & 8 != 0 {
        // 后台运行
        Ok(None)
    } else {
        // 正常模式
        let status = child.wait()?;
        if status.success() {
            Ok(None)
        } else if mode & 2 == 0 {
            //未关闭错误输出才返回错误
            Err(RuntimeError::CommandFailed2(
                cmdstr.to_owned(),
                status.to_string(),
            ))
        } else {
            Ok(None)
        }
    }
}

// 管道
pub fn handle_command(
    cmd: &String,
    args: &Vec<Expression>,
    state: &mut State,
    env: &mut Environment,
    depth: usize,
) -> Result<Expression, RuntimeError> {
    let always_pipe = state.contains(State::IN_PIPE);
    let mut cmd_args = vec![];
    for arg in args {
        // for flattened_arg in Expression::flatten(vec![arg.eval_mut(env, depth + 1)?]) {
        state.set(State::SKIP_BUILTIN_SEEK);
        let e_arg = arg.eval_mut(state, env, depth)?;
        state.clear(State::SKIP_BUILTIN_SEEK);
        match e_arg {
            Expression::Symbol(s) => cmd_args.push(s.to_string()),
            Expression::String(mut s) => {
                if s.starts_with("~") {
                    if let Some(home_dir) = dirs::home_dir() {
                        s = s.replace("~", home_dir.to_string_lossy().as_ref());
                    }
                }
                if s.contains('*') {
                    let mut matched = false;
                    for path in glob(&s).unwrap().filter_map(Result::ok) {
                        matched = true;
                        cmd_args.push(path.to_string_lossy().to_string());
                    }
                    if !matched {
                        return Err(RuntimeError::WildcardNotMatched(s));
                        // cmd_args.push(s);
                    }
                } else {
                    cmd_args.push(s)
                }
            }
            Expression::Bytes(b) => cmd_args.push(String::from_utf8_lossy(&b).to_string()),
            Expression::None => continue,
            _ => cmd_args.push(format!("{}", e_arg)),
        }
    }
    let cmd_mode: u8 = match cmd_args.last() {
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
    };
    // dbg!(args, &cmd_args);
    let last_input = state.pipe_out();
    let pipe_input = to_bytes(last_input);
    let result = exec_single_cmd(
        cmd,
        Some(&cmd_args),
        env,
        Some(pipe_input),
        always_pipe,
        cmd_mode,
    )?;
    Ok(to_expr(result))
}

pub fn to_expr(bytes_out: Option<Vec<u8>>) -> Expression {
    match bytes_out {
        Some(b) => Expression::String(String::from_utf8_lossy(&b).trim().to_string()),
        _ => Expression::None,
    }
}
fn to_bytes(expr_out: Option<Expression>) -> Vec<u8> {
    match expr_out {
        Some(p) => p.to_string().as_bytes().to_owned(),
        _ => vec![],
    }
}
