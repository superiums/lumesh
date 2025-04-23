use crate::{Environment, Expression, LmError};

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
) -> Result<(Vec<u8>, Expression), LmError> {
    dbg!("------ exec:------", &cmdstr, &args, &is_last);
    let mut cmd = Command::new(&cmdstr);
    cmd.args(args).envs(bindings);
    // 设置 stdin
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::inherit());
    }

    // 设置 stdout（如果是交互式命令，直接接管终端）
    let is_interactive = is_interactive();
    if is_last && is_interactive {
        cmd.stdout(Stdio::inherit());
    } else if !is_last {
        cmd.stdout(Stdio::piped());
    }

    // 执行命令
    let mut child = cmd.spawn().map_err(|e| match &e.kind() {
        ErrorKind::NotFound => LmError::ProgramNotFound(cmdstr),
        _ => LmError::CommandFailed2(cmdstr, e.to_string()),
    })?;

    // 写入输入（如果不是第一条命令）
    if let Some(input) = input {
        child.stdin.as_mut().unwrap().write_all(input)?;
    }

    // 获取输出（如果不是最后一条命令）
    if !is_last || !is_interactive {
        let output = child.wait_with_output()?;
        let expr = Expression::Bytes(output.stdout.clone());
        Ok((output.stdout, expr))
    } else {
        child.wait()?;
        Ok((vec![], Expression::None))
    }
}

fn expr_to_command(
    expr: &Expression,
    env: &mut Environment,
) -> Result<(String, Vec<String>), LmError> {
    // let bindings = env.get_bindings_map();

    match expr {
        Expression::Symbol(name) => {
            let cmd_name = match env.get(name) {
                Some(Expression::Symbol(alias)) => alias,
                Some(_) => return Err(LmError::ProgramNotFound(name.clone())),
                None => name.clone(),
            };
            Ok((cmd_name, vec![]))
        }
        Expression::Apply(func, args) => {
            /* 处理函数调用 */
            dbg!("applying in pipe:", func, args);

            // 分派到具体类型处理
            return match *func.clone() {
                // | Self::String(name)
                Expression::Symbol(name) | Expression::String(name) => {
                    let cmd_args: Vec<String> = args.iter().map(|expr| expr.to_string()).collect();
                    Ok((name, cmd_args))
                }
                _ => {
                    dbg!("--else type--", &func);
                    Err(LmError::ProgramNotFound(expr.to_string()))
                }
            };
        }
        _ => Err(LmError::ProgramNotFound(expr.to_string())),
    }
}

// 管道
pub fn handle_pipes(
    lhs: Box<Expression>,
    rhs: Box<Expression>,
    bindings: &BTreeMap<String, String>,
    has_right: bool,
    input: Option<&[u8]>, // 前一条命令的输出（None 表示第一个命令）
    env: &mut Environment,
    depth: usize,
) -> Result<(Vec<u8>, Expression), LmError> {
    {
        // 管道运算符特殊处理
        // dbg!("--pipe--", &lhs, &rhs);
        let result_left = match *lhs {
            Expression::BinaryOp(op, l_arm, r_arm) if op == "|" => {
                handle_pipes(l_arm, r_arm, bindings, true, input, env, depth)
            }
            _ => {
                let (cmd, args) = expr_to_command(&lhs, env)?;
                exec_single_cmd(cmd, args, bindings, input, false)
            }
        };
        return match result_left {
            Ok((pipe_out, _)) => {
                return match *rhs {
                    Expression::BinaryOp(op, l_arm, r_arm) if op == "|" => handle_pipes(
                        l_arm,
                        r_arm,
                        bindings,
                        has_right,
                        Some(&pipe_out),
                        env,
                        depth,
                    ),
                    _ => {
                        let (cmd, args) = expr_to_command(&rhs, env)?;
                        exec_single_cmd(cmd, args, bindings, Some(&pipe_out), !has_right)
                    }
                };
            }
            Err(e) => Err(e),
        };
    }
}
