use crate::{Environment, Expression, RuntimeError};

use std::{
    collections::BTreeMap,
    io::{ErrorKind, Write},
    process::{Command, Stdio},
};

fn is_interactive() -> bool {
    return true;
}
/// 执行单个命令（支持管道）
fn exec_single_cmd(
    cmdstr: String,
    args: Vec<String>,
    bindings: &BTreeMap<String, String>,
    input: Option<&[u8]>, // 前一条命令的输出（None 表示第一个命令）
    is_last: bool,        // 是否是最后一条命令？
    always_pipe: bool,
) -> Result<(Vec<u8>, Expression), RuntimeError> {
    // dbg!("------ exec:------", &cmdstr, &args, &is_last);
    let mut cmd = Command::new(&cmdstr);
    cmd.args(args)
        .envs(bindings)
        .current_dir(std::env::current_dir()?);
    // 设置 stdin
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::inherit());
    }

    // 设置 stdout（如果是交互式命令，直接接管终端）
    let is_interactive = is_interactive();
    if always_pipe {
        cmd.stdout(Stdio::piped());
    } else if is_last && is_interactive {
        cmd.stdout(Stdio::inherit());
    } else {
        cmd.stdout(Stdio::piped());
    }

    // 执行命令
    let mut child = cmd.spawn().map_err(|e| match &e.kind() {
        ErrorKind::NotFound => RuntimeError::ProgramNotFound(cmdstr),
        _ => RuntimeError::CommandFailed2(cmdstr, e.to_string()),
    })?;

    // 写入输入（如果不是第一条命令）
    if let Some(input) = input {
        child.stdin.as_mut().unwrap().write_all(input)?;
    }

    // 获取输出（如果不是最后一条命令）
    if always_pipe || !is_last || !is_interactive {
        let output = child.wait_with_output()?;
        let expr = Expression::String(String::from_utf8_lossy(&output.stdout).into_owned());
        // dgb!(cmd.get_program(), &expr);
        Ok((output.stdout, expr))
    } else {
        child.wait()?;
        Ok((vec![], Expression::None))
    }
}

/// 遇到外部命令则返回命令和参数，其他可执行命令返回表达式，否则返回错误
fn expr_to_command(
    expr: &Expression,
    env: &mut Environment,
    depth: usize,
) -> Result<(String, Vec<String>, Option<Expression>), RuntimeError> {
    // let bindings = env.get_bindings_map();

    match expr {
        // 无参数的外部命令，如ls
        Expression::Symbol(name) => {
            let cmd_name = match env.get(name) {
                Some(Expression::Symbol(alias)) => alias,
                Some(_) => return Err(RuntimeError::ProgramNotFound(name.clone())),
                None => name.clone(),
            };
            Ok((cmd_name, vec![], None))
        }
        Expression::Apply(func, args) => {
            /* 处理函数调用，如 3+5 */
            // dbg!("applying in pipe:", func, args);
            let func_eval = func.clone().eval_mut(env, depth + 1)?;

            // 得到执行后的实际命令
            return match func_eval {
                // 是外部命令+参数，
                Expression::Symbol(name) | Expression::String(name) => {
                    let cmd_args: Vec<String> = args.iter().map(|expr| expr.to_string()).collect();
                    Ok((name, cmd_args, None))
                }
                // 其他可执行命令，如lambda,Function,Builtin
                _ => {
                    // dgb!("--else type--", &func_eval, &func_eval.type_name());
                    Ok(("".into(), vec![], Some(expr.to_owned())))

                    // Err(RuntimeError::ProgramNotFound(func_eval.to_string()))
                }
            };
        }
        _ => Err(RuntimeError::ProgramNotFound(expr.to_string())),
    }
}

// 管道
pub fn handle_pipes(
    lhs: &Expression,
    rhs: &Expression,
    bindings: &BTreeMap<String, String>,
    has_right: bool,
    input: Option<&[u8]>, // 前一条命令的输出（None 表示第一个命令）
    env: &mut Environment,
    depth: usize,
    always_pipe: bool,
) -> Result<(Vec<u8>, Expression), RuntimeError> {
    {
        // 管道运算符特殊处理
        // dbg!("--pipe--", &lhs, &rhs);
        let result_left = match lhs {
            Expression::BinaryOp(op, l_arm, r_arm) if op == "|" => handle_pipes(
                &*l_arm,
                &*r_arm,
                bindings,
                true,
                input,
                env,
                depth,
                always_pipe,
            ),
            _ => {
                let (cmd, args, expr) = expr_to_command(&lhs, env, depth)?;
                if expr.is_some() {
                    // 有表达式返回则执行表达式
                    let result_expr = expr.unwrap().eval_apply(env, depth)?;
                    let result_expr_bytes = result_expr.to_string().as_bytes().to_owned();
                    Ok((result_expr_bytes, result_expr))
                } else {
                    // 否则执行外部command
                    exec_single_cmd(cmd, args, bindings, input, false, always_pipe)
                }
            }
        };
        return match result_left {
            Ok((pipe_out, _)) => {
                return match rhs {
                    Expression::BinaryOp(op, l_arm, r_arm) if op == "|" => handle_pipes(
                        &*l_arm,
                        &*r_arm,
                        bindings,
                        has_right,
                        Some(&pipe_out),
                        env,
                        depth,
                        always_pipe,
                    ),
                    _ => {
                        let (cmd, args, expr) = expr_to_command(&rhs, env, depth)?;
                        match expr {
                            Some(ex) => {
                                let result_expr = ex.eval_apply(env, depth)?;
                                let result_expr_bytes =
                                    result_expr.to_string().as_bytes().to_owned();
                                Ok((result_expr_bytes, result_expr))
                            }
                            _ => {
                                let result_right = exec_single_cmd(
                                    cmd.clone(),
                                    args,
                                    bindings,
                                    Some(&pipe_out),
                                    !has_right,
                                    always_pipe,
                                );
                                // dgb!(&cmd, &result_right);
                                result_right
                            }
                        }
                    }
                };
            }
            Err(e) => Err(e),
        };
    }
}

// 输入重定向处理
pub fn handle_stdin_redirect(
    lhs: Expression,
    rhs: Expression,
    env: &mut Environment,
    depth: usize,
    always_pipe: bool,
) -> Result<Expression, RuntimeError> {
    // 读取
    let path = rhs.eval_mut(env, depth + 1)?.to_string();
    let contents = std::fs::read(path)?;
    // 左侧
    let (cmd, args, expr) = expr_to_command(&lhs, env, depth)?;
    let bindings = env.get_bindings_map();
    if expr.is_some() {
        // lambda, fn, builtin may read stdin?
        Err(RuntimeError::CustomError(format!(
            "expr {expr:?} can't read stdin"
        )))
    } else {
        // 否则执行外部command
        let (_, result) =
            exec_single_cmd(cmd, args, &bindings, Some(&contents), true, always_pipe)?;
        Ok(result)
    }
}
