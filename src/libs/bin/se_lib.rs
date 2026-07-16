use std::{
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
};

use regex_lite::Regex;

use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind,
    eval::State,
    libs::{
        BuiltinInfo, SelfExpandFunc,
        helper::{
            check_args_len, check_exact_args_len, get_integer_arg, get_string_arg, get_table_arg,
        },
    },
    reg_info,
};
static FORMAT_RE: OnceLock<Regex> = OnceLock::new();

pub fn regist_se() -> HashMap<&'static str, SelfExpandFunc> {
    let mut module: HashMap<&'static str, SelfExpandFunc> = HashMap::new();
    module.insert("format", format);
    module.insert("where", r#where);
    module.insert("repeat", repeat);
    module.insert("assert", assert);
    module.insert("when", when);
    module.insert("debug", debug);
    module.insert("ddebug", ddebug);
    module.insert("typeof", r#typeof);
    module.insert("set_root", set_root);
    module.insert("unset_root", unset_root);
    module
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
      // debug
      when => "conditional execute", "<condition> <execute>"
      assert => "assert condition is true, throw error if false", "<condition> [message]"
      debug => "print debug representation", "<args>..."
      ddebug => "print pretty debug", "<args>..."
      typeof => "get type of data value", "<value>"

      // Data manipulation
      format => "print formatted string with vars", "<template> <args>..."
      where => "filter rows by condition", "<table> <condition> "

      // Execution control
      repeat => "evaluate expr n times", "<expr> <n>"

      // env
      set_root => "define a variable in root environment", "<var> <val>"
      unset_root => "undefine a variable in root environment", "<var>"
      // getvar => "get a variable value", "<var>"

    })
}

// args should be lazy evaled.
fn r#where(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("where", args, 2, ctx)?;
    let data_evaled = args[0].eval_mut(state, env, 0)?;
    let data = get_table_arg(data_evaled, ctx)?;

    let is_last_local = state.contains(State::IN_LOCAL);
    let last_local_vars = if is_last_local {
        Some(state.get_local_vars())
    } else {
        None
    };
    state.set(State::IN_LOCAL);

    let predicate = |nr: usize, row: &[Expression]| -> bool {
        state.set_local_var("NR".to_string(), Expression::Integer(nr as i64));
        for (nf, cell) in row.iter().enumerate() {
            let name = data
                .headers()
                .get(nf)
                .map_or("unkown".to_string(), |x| x.to_string());
            state.set_local_var(name, cell.clone());
            state.set_local_var("NF".to_string(), Expression::Integer(nf as i64));
        }
        match args[1].eval_mut(state, env, 0) {
            Ok(x) => x.is_truthy(),
            _ => false,
        }
    };

    let filtered = data.filter_rows(predicate);

    if is_last_local {
        state.set_local_vars(last_local_vars.unwrap());
    } else {
        state.clear_local_var();
        state.clear(State::IN_LOCAL);
    }

    Ok(Expression::Table(filtered))
}

// args should be lazy evaled
fn repeat(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("repeat", args, 2, ctx)?;
    let n = get_integer_arg(args[1].eval_mut(state, env, 0)?, ctx)?;
    let results = (0..n)
        .map(|_| args[0].eval_with_assign(state, env))
        .collect::<Result<Vec<_>, _>>()?;
    if results.iter().any(|x| x != &Expression::None) {
        Ok(Expression::from(results))
    } else {
        Ok(Expression::None)
    }
}

fn assert(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("assert", &args, 1..=2, ctx)?;

    let condition = args[0].eval_with_assign(state, env)?;
    let is_true = condition.is_truthy();

    if !is_true {
        let message = if args.len() > 1 {
            args[1].eval_with_assign(state, env)?.to_string()
        } else {
            "assertion failed".to_string()
        };

        return Err(RuntimeError::new(
            RuntimeErrorKind::CustomError(message.into()),
            ctx.clone(),
            0,
        ));
    }

    Ok(Expression::None)
}

fn when(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("when", &args, 2, ctx)?;

    if args[0].eval_with_assign(state, env)?.is_truthy() {
        return args[1].eval_with_assign(state, env);
    }

    Ok(Expression::None)
}

// args lazy
fn debug(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut results = Vec::new();
    for x in args.iter() {
        let expr_repr = format!("{x:?}");
        let y = x.eval_with_assign(state, env)?;
        let mut map = BTreeMap::new();
        map.insert("expr".to_string(), Expression::String(expr_repr));
        map.insert("type".to_string(), Expression::String(y.type_name()));
        map.insert("value".to_string(), y);
        results.push(Expression::from(map));
    }
    Ok(Expression::from(results))
}

// args lazy
fn ddebug(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let mut results = Vec::new();
    for x in args.iter() {
        let expr_repr = format!("{x:#}");
        let y = x.eval_with_assign(state, env)?;
        let mut map = BTreeMap::new();
        map.insert("expr".to_string(), Expression::String(expr_repr));
        map.insert("type".to_string(), Expression::String(y.type_name()));
        map.insert("value".to_string(), y);
        results.push(Expression::from(map));
    }
    Ok(Expression::from(results))
}

// arg lazy
fn r#typeof(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("typeof", args, 1, ctx)?;
    let t = args[0].eval_with_assign(state, env)?.type_name();
    Ok(Expression::from(t))
}

// Print Formated
fn format(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("format", args, 1.., ctx)?;
    let template_expr = args[0].eval_mut(state, env, 0)?;
    let template = get_string_arg(template_expr, ctx)?;
    let re = FORMAT_RE.get_or_init(|| Regex::new(r#"\{(\w+)\}"#).unwrap());
    let mut result = template.clone();
    for (full, [var]) in re.captures_iter(&template).map(|m| m.extract()) {
        let value = ctx.handle_variable(var, false, state, env, 0)?;
        result = result.replace(full, &value.to_string());
    }

    // position arg
    let placeholders = result.matches("{}").count();
    for arg in args.iter().skip(1).take(placeholders) {
        result = result.replacen("{}", &arg.eval_mut(state, env, 0)?.to_string(), 1);
    }

    // println!("{}", result);
    Ok(Expression::String(result))
}

// lazy arg
pub fn set_root(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set_root", args, 2, ctx)?;
    let name = args[0].to_string();
    let expr = args[1].eval_with_assign(state, env)?;
    env.define_in_root(&name, expr);
    Ok(Expression::None)
}

// lazy arg
pub fn unset_root(
    args: &[Expression],
    env: &mut Environment,
    _state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("unset_root", args, 1, ctx)?;
    let name = args[0].to_string();
    env.undefine_in_root(&name);
    Ok(Expression::None)
}
