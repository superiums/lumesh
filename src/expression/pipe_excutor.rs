use crate::{Environment, Expression, RuntimeError};

use std::{
    io::{ErrorKind, Write},
    process::{Command, Stdio},
};

use super::eval::State;

fn is_interactive() -> bool {
    true
}
/// 执行单个命令（支持管道）
pub fn exec_single_cmd(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>, // 前一条命令的输出（None 表示第一个命令）
    is_last: bool,          // 是否是最后一条命令？
    always_pipe: bool,
) -> Result<Vec<u8>, RuntimeError> {
    // dbg!("------ exec:------", &cmdstr, &args, &is_last);
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
        ErrorKind::NotFound => RuntimeError::ProgramNotFound(cmdstr.clone()),
        _ => RuntimeError::CommandFailed2(cmdstr.clone(), e.to_string()),
    })?;

    // 写入输入（如果不是第一条命令）
    if let Some(input) = input {
        child.stdin.as_mut().unwrap().write_all(&input)?;
    }

    // 获取输出（如果不是最后一条命令）
    if always_pipe || !is_last || !is_interactive {
        let output = child.wait_with_output()?;
        // let expr = Expression::String(String::from_utf8_lossy(&output.stdout).into_owned());
        // dgb!(cmd.get_program(), &expr);
        Ok(output.stdout)
    } else {
        child.wait()?;
        Ok(vec![])
    }
}

/// 遇到外部命令则返回命令和参数，其他可执行命令返回表达式，否则返回错误
// fn expr_to_command<'a>(
//     expr: &'a Expression,
//     state: &mut State,
//     env: &mut Environment,
//     depth: usize,
// ) -> Result<
//     (
//         Option<String>,
//         Option<Vec<String>>,
//         Option<&'a Expression>,
//         Option<(&'a CatchType, &'a Option<Rc<Expression>>)>,
//     ),
//     RuntimeError,
// > {
//     // let bindings = env.get_bindings_map();

//     match expr {
//         // 无参数的外部命令，如ls
//         Expression::Symbol(name) => {
//             let cmd_name = match env.get(name) {
//                 Some(Expression::Symbol(alias)) => alias,
//                 Some(_) => return Err(RuntimeError::ProgramNotFound(name.clone())),
//                 None => name.clone(),
//             };
//             Ok((Some(cmd_name), None, None, None))
//         }
//         Expression::Apply(func, _) => {
//             /* 处理函数调用，如 3+5 */
//             // dbg!("applying in pipe:", func, args);
//             let func_eval = func.as_ref().eval_mut(state, env, depth + 1)?;

//             // 得到执行后的实际命令
//             match func_eval {
//                 // 是外部命令+参数，
//                 Expression::Symbol(name) | Expression::String(name) => {
//                     // let cmd_args: Vec<String> = args.iter().map(|expr| expr.to_string()).collect();
//                     // Ok((name, cmd_args, None))
//                     Err(RuntimeError::CustomError(format!(
//                         "cant't apply symbol {}: {} as cmd",
//                         &func, name
//                     )))
//                 }
//                 // 其他可执行命令，如lambda,Function,Builtin
//                 _ => {
//                     // dgb!("--else type--", &func_eval, &func_eval.type_name());
//                     Ok((None, None, Some(expr), None))

//                     // Err(RuntimeError::ProgramNotFound(func_eval.to_string()))
//                 }
//             }
//         }
//         Expression::Command(func, args) => {
//             /* 处理函数调用，如 3+5 */
//             // dbg!("applying in pipe:", func, args);
//             let func_eval = func.as_ref().eval_mut(state, env, depth + 1)?;

//             // 得到执行后的实际命令
//             match func_eval {
//                 // 是外部命令+参数，
//                 Expression::Symbol(name) | Expression::String(name) => {
//                     let cmd_args: Vec<String> = args.iter().map(|expr| expr.to_string()).collect();
//                     Ok((Some(name), Some(cmd_args), None, None))
//                 }
//                 // 其他可执行命令，如lambda,Function,Builtin
//                 _ => {
//                     // dbg!("--else type--", &func_eval, &func_eval.type_name());
//                     Ok((None, None, Some(expr), None))

//                     // Err(RuntimeError::ProgramNotFound(func_eval.to_string()))
//                 }
//             }
//         }
//         Expression::Pipe(..)
//         | Expression::BinaryOp(..)
//         | Expression::UnaryOp(..)
//         | Expression::Integer(..)
//         | Expression::Float(..)
//         | Expression::String(..)
//         | Expression::Boolean(..)
//         | Expression::List(..)
//         | Expression::HMap(..)
//         | Expression::Map(..)
//         | Expression::Index(..)
//         | Expression::Slice(..) => Ok((None, None, Some(expr), None)),
//         // 是分组则解开后再次解释
//         Expression::Group(inner) => expr_to_command(inner, state, env, depth + 1),
//         Expression::Catch(body, typ, deeling) => {
//             // dbg!(&typ, &deeling);
//             let (body_name, body_arg, body_expr, _) = expr_to_command(body, state, env, depth + 1)?;
//             Ok((body_name, body_arg, body_expr, Some((typ, deeling))))
//         }
//         _ => Err(RuntimeError::ProgramNotFound(expr.to_string())),
//     }
// }

// 管道
pub fn handle_command(
    cmd: &String,
    args: &Vec<Expression>,
    // bindings: &HashMap<String, String>,
    // has_right: bool,
    // input: Option<&[u8]>, // 前一条命令的输出（None 表示第一个命令）
    state: &mut State,
    env: &mut Environment,
    depth: usize,
    // always_pipe: bool,
) -> Result<Expression, RuntimeError> {
    // let bindings = env.get_bindings_map();
    // let always_pipe = env.has("__ALWAYSPIPE");
    let always_pipe = state.contains(State::IN_PIPE);
    let mut cmd_args = vec![];
    for arg in args {
        // for flattened_arg in Expression::flatten(vec![arg.eval_mut(env, depth + 1)?]) {
        let e_arg = arg.eval_mut(state, env, depth)?;
        match e_arg {
            Expression::String(s) => cmd_args.push(s),
            Expression::Bytes(b) => cmd_args.push(String::from_utf8_lossy(&b).to_string()),
            Expression::None => continue,
            _ => cmd_args.push(format!("{}", e_arg)),
        }
    }
    // dbg!(args, &cmd_args);
    let last_input = state.pipe_out();
    let pipe_input = to_bytes(last_input);
    let result = exec_single_cmd(
        cmd,
        Some(cmd_args),
        env,
        Some(pipe_input),
        true,
        always_pipe,
    )?;
    Ok(to_expr(Some(result)))
}

// 管道
// pub fn handle_pipes<'a>(
//     lhs: &Expression,
//     rhs: &Expression,
//     // bindings: &HashMap<String, String>,
//     has_right: bool,
//     input: Option<Vec<u8>>,         // 前一条命令的输出（None 表示第一个命令）
//     expr_input: Option<Expression>, // 前一条命令的输出（None 表示第一个命令）
//     state: &mut State,
//     env: &mut Environment,
//     depth: usize,
//     always_pipe: bool,
// ) -> Result<(Option<Vec<u8>>, Option<Expression>), RuntimeError> {
//     {
//         // 管道运算符特殊处理
//         // dbg!("--pipe--", &lhs, &rhs);
//         let result_left = match lhs {
//             // TODO op== "|>" >> >>>
//             Expression::Pipe(op, l_arm, r_arm) if op == "|" => handle_pipes(
//                 l_arm,
//                 r_arm,
//                 // bindings,
//                 true,
//                 input,
//                 expr_input,
//                 state,
//                 env,
//                 depth + 1,
//                 always_pipe,
//             ),

//             _ => {
//                 let (cmd, args, expr, deeling) = expr_to_command(lhs, state, env, depth + 1)?;
//                 // dbg!(&cmd, &args, &expr, &deeling);
//                 // 有表达式返回则执行表达式, 有apply和binaryOp,catch三种,还有从group解开的pipe
//                 match expr {
//                     Some(Expression::Pipe(op, l_arm, r_arm)) if op == "|" => {
//                         let result = handle_pipes(
//                             l_arm,
//                             r_arm,
//                             // bindings,
//                             true,
//                             input,
//                             expr_input,
//                             state,
//                             env,
//                             depth + 1,
//                             always_pipe,
//                         );
//                         match result {
//                             Ok(r) => Ok(r),
//                             Err(e) => handle_err(e, expr.unwrap(), deeling, state, env, depth),
//                         }
//                     }
//                     Some(ex) => {
//                         // let result_expr = expr.unwrap().eval_apply(env, depth)?;
//                         let result_expr = ex.eval_mut(state, env, depth + 1);
//                         match result_expr {
//                             Ok(r) => Ok((None, Some(r))),
//                             Err(e) => handle_err(e, ex, deeling, state, env, depth),
//                         }
//                     }
//                     None => {
//                         // 否则执行外部command
//                         if let Some(cmdx) = cmd {
//                             let result_pipe =
//                                 exec_single_cmd(&cmdx, args, env, input, false, always_pipe);
//                             match result_pipe {
//                                 Ok(r) => Ok((Some(r), None)),
//                                 Err(e) => handle_err(
//                                     e,
//                                     &Expression::String(cmdx),
//                                     deeling,
//                                     state,
//                                     env,
//                                     depth,
//                                 ),
//                             }
//                         } else {
//                             Err(RuntimeError::CustomError(
//                                 "No Expression or Command found".into(),
//                             ))
//                         }
//                     }
//                 }
//                 // dbg!(&left_run, &deeling);
//                 // match deeling {
//                 //     Some((ctyp, handler)) => match left_run {
//                 //         Ok(r) => {
//                 //             dbg!("left ok");
//                 //             Ok(r)
//                 //         }
//                 //         Err(e) => {
//                 //             dbg!("left err, deeling");
//                 //             let exd = catch_error(
//                 //                 e,
//                 //                 Box::new(if expr.is_some() {
//                 //                     expr.unwrap()
//                 //                 } else {
//                 //                     Expression::String(cmd)
//                 //                 }),
//                 //                 ctyp,
//                 //                 handler,
//                 //                 env,
//                 //             )?;
//                 //             Ok((None, Some(exd)))
//                 //         }
//                 //     },
//                 //     _ => {
//                 //         dbg!("no deeling");
//                 //         left_run
//                 //     }
//                 // }
//             }
//         };
//         // dbg!(&result_left);
//         match result_left {
//             Ok((pipe_out, expr_out)) => {
//                 // return match rhs {
//                 //     Expression::Pipe(op, l_arm, r_arm) if op == "|" => handle_pipes(
//                 //         &*l_arm,
//                 //         &*r_arm,
//                 //         bindings,
//                 //         has_right,
//                 //         pipe_out,
//                 //         expr_out,
//                 //         env,
//                 //         depth,
//                 //         always_pipe,
//                 //     ),
//                 //     _ => {
//                 let (cmd, args, expr, deeling) = expr_to_command(rhs, state, env, depth + 1)?;

//                 match expr {
//                     // 有表达式返回则执行表达式, 有apply和binaryOp,catch三种
//                     Some(ex) => {
//                         match ex {
//                             Expression::Apply(..) => {
//                                 // 右侧是函数，读取左侧的算术结果
//                                 // 如果算术结果为空，则从标准输出的结果转换
//                                 let choosed_input = match expr_out {
//                                     Some(o) => o,
//                                     _ => to_expr(pipe_out),
//                                 };
//                                 // dbg!(&choosed_input);
//                                 let result_expr = ex.append_args(vec![choosed_input]).eval_apply(
//                                     // true,
//                                     state,
//                                     env,
//                                     depth + 1,
//                                 );
//                                 match result_expr {
//                                     Ok(r) => Ok((None, Some(r))),
//                                     Err(e) => handle_err(e, ex, deeling, state, env, depth),
//                                 }
//                             }
//                             Expression::Pipe(op, l_arm, r_arm) if op == "|" => {
//                                 let result = handle_pipes(
//                                     l_arm,
//                                     r_arm,
//                                     // bindings,
//                                     has_right,
//                                     pipe_out,
//                                     expr_out,
//                                     state,
//                                     env,
//                                     depth + 1,
//                                     always_pipe,
//                                 );
//                                 match result {
//                                     Ok(r) => Ok(r),
//                                     Err(e) => handle_err(
//                                         e,
//                                         &Expression::Pipe(op.clone(), l_arm.clone(), r_arm.clone()),
//                                         deeling,
//                                         state,
//                                         env,
//                                         depth,
//                                     ),
//                                 }
//                             }
//                             _ => {
//                                 // 右侧为binop？报错？不能接收收入!!
//                                 let result_expr = ex.eval_mut(state, env, depth + 1);
//                                 match result_expr {
//                                     Ok(r) => Ok((None, Some(r))),
//                                     Err(e) => handle_err(e, ex, deeling, state, env, depth),
//                                 }
//                             }
//                         }
//                     }
//                     _ => {
//                         // 右侧是命令，读取左侧的标准输出结果
//                         // 如果标准输出结果为空，则从算术运算的结果转换
//                         if let Some(cmdx) = cmd {
//                             let choosed_input = match pipe_out {
//                                 Some(po) => po,
//                                 _ => to_bytes(expr_out),
//                             };
//                             let result_right = exec_single_cmd(
//                                 &cmdx,
//                                 args,
//                                 env,
//                                 Some(choosed_input),
//                                 !has_right,
//                                 always_pipe,
//                             )?;
//                             // dgb!(&cmd, &result_right);
//                             Ok((Some(result_right), None))
//                         } else {
//                             Err(RuntimeError::CustomError(
//                                 "No Expression or Command found".into(),
//                             ))
//                         }
//                     }
//                 }
//                 // match deeling {
//                 //     Some((ctyp, handler)) => match right_run {
//                 //         Ok(r) => Ok(r),
//                 //         Err(e) => {
//                 //             let exd = catch_error(
//                 //                 e,
//                 //                 Box::new(if expr.is_some() {
//                 //                     expr.unwrap()
//                 //                 } else {
//                 //                     Expression::String(cmd)
//                 //                 }),
//                 //                 ctyp,
//                 //                 handler,
//                 //                 env,
//                 //             )?;
//                 //             Ok((None, Some(exd)))
//                 //         }
//                 //     },
//                 //     _ => right_run,
//                 // }
//             }
//             Err(e) => Err(e),
//         }
//     }
// }

// fn handle_err(
//     e: RuntimeError,
//     body: &Expression,
//     deeling: Option<(&CatchType, &Option<Rc<Expression>>)>,
//     state: &mut State,
//     env: &mut Environment,
//     depth: usize,
// ) -> Result<(Option<Vec<u8>>, Option<Expression>), RuntimeError> {
//     match deeling {
//         Some((ctyp, handler)) => {
//             // dbg!("left err, deeling");
//             let exd = catch_error(
//                 e,
//                 &Rc::new(body.clone()),
//                 ctyp,
//                 handler,
//                 state,
//                 env,
//                 depth + 1,
//             )?;
//             Ok((None, Some(exd)))
//         }
//         _ => Err(e),
//     }
// }

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
// 输入重定向处理
// pub fn handle_stdin_redirect(
//     lhs: &Expression,
//     rhs: &Expression,
//     state: &mut State,
//     env: &mut Environment,
//     depth: usize,
//     always_pipe: bool,
// ) -> Result<Expression, RuntimeError> {
//     // 读取
//     let path = rhs.eval_mut(state, env, depth + 1)?.to_string();
//     let contents = std::fs::read(path)?;
//     // 左侧
//     let (cmd, args, expr, deeling) = expr_to_command(lhs, state, env, depth)?;
//     // let bindings = env.get_bindings_map();
//     if expr.is_some() {
//         // lambda, fn, builtin may read stdin?
//         Err(RuntimeError::CustomError(format!(
//             "expr {expr:?} can't read stdin"
//         )))
//     } else if let Some(cmdx) = cmd {
//         // 否则执行外部command

//         let result = exec_single_cmd(&cmdx, args, env, Some(contents), true, always_pipe);
//         match result {
//             Ok(r) => Ok(to_expr(Some(r))),
//             Err(e) => match deeling {
//                 Some((ctyp, handler)) => {
//                     // dbg!("left err, deeling");
//                     let exd = catch_error(
//                         e,
//                         &Rc::new(Expression::String(cmdx)),
//                         ctyp,
//                         handler,
//                         state,
//                         env,
//                         depth + 1,
//                     )?;
//                     Ok(exd)
//                 }
//                 _ => Err(e),
//             }, // handle_err(e, Expression::String(cmd), deeling, env),
//         }
//     } else {
//         Err(RuntimeError::CustomError(
//             "no expression nor cmd found".into(),
//         ))
//     }
// }
