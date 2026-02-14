use std::collections::{BTreeMap, HashMap};

use crate::{
    Environment, Expression, RuntimeError, RuntimeErrorKind,
    eval::State,
    libs::{
        BuiltinInfo, SelfExpandFunc,
        bin::list_lib::get_list_ref,
        helper::{check_args_len, check_exact_args_len, get_integer_arg, get_string_arg},
    },
    reg_info,
};

pub fn regist_se() -> HashMap<&'static str, SelfExpandFunc> {
    let mut module: HashMap<&'static str, SelfExpandFunc> = HashMap::new();
    module.insert("printf", printf);
    module.insert("where", r#where);
    module.insert("repeat", repeat);
    module.insert("debug", debug);
    module.insert("ddebug", ddebug);
    module.insert("typeof", r#typeof);
    module.insert("set_root", set_root);
    module.insert("unset_root", unset_root);
    module
}

pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
      // IO
      printf => "print formatted string with vars", "<template> <args>..."
      debug => "print debug representation", "<args>..."
      ddebug => "print pretty debug", "<args>..."

      // Data manipulation
      typeof => "get data type", "<value>"
      where => "filter rows by condition", "<list[map/list/set]> <condition> "

      // Execution control
      repeat => "evaluate expr n times", "<expr> <n>"

      // env
      set_root => "define a variable in root environment", "<var> <val>"
      unset_root => "undefine a variable in root environment", "<var>"
      getvar => "get a variable value", "<var>"

    })
}

// args should be lazy evaled.
fn r#where(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("where", &args, 2, ctx)?;
    let list = args[0].eval_mut(state, env, 0)?;
    let list = get_list_ref(&list, ctx)?;

    let mut filtered = Vec::new();
    state.set(State::IN_LOCAL);
    for (i, row) in list.iter().enumerate() {
        state.set_local_var("NR".to_string(), Expression::Integer(i as i64));
        if let Expression::HMap(row_map) = row {
            state.set_local_vars(row_map.as_ref().clone());
        } else if let Expression::Map(row_map) = row {
            for (k, v) in row_map.as_ref() {
                state.set_local_var(k.to_string(), v.clone());
            }
        } else if let Expression::List(row_set) = row {
            for (nf, item) in row_set.iter().enumerate() {
                state.set_local_var("NF".to_string(), Expression::Integer(nf as i64));
                state.set_local_var("F".to_string(), item.clone());
            }
        } else if let Expression::BSet(row_set) = row {
            for (nf, item) in row_set.iter().enumerate() {
                state.set_local_var("NF".to_string(), Expression::Integer(nf as i64));
                state.set_local_var("F".to_string(), item.clone());
            }
        } else {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeError {
                    expected: "Map/HMap/List/Set as Field".to_string(),
                    found: row.type_name(),
                    sym: row.to_string(),
                },
                ctx.clone(),
                0,
            ));
        }

        let c = args[1].eval_mut(state, env, 0)?;
        if let Expression::Boolean(true) = c {
            filtered.push(row.clone());
        }
    }
    Ok(Expression::from(filtered))
}

// args should be lazy evaled
fn repeat(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("repeat", &args, 2, ctx)?;
    let n = get_integer_arg(args[1].eval_mut(state, env, 0)?, ctx)?;
    let r = (0..n)
        .map(|_| args[0].eval_with_assign(state, env))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expression::from(r))
}

// args lazy
fn debug(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    for x in args.iter() {
        println!("{x:?}");
        println!("-->");
        let y = x.eval_with_assign(state, env)?;
        println!("{y:?}");
        println!()
    }
    Ok(Expression::None)
}

// args lazy
fn ddebug(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    for x in args.iter() {
        println!("{x:#}");
        println!("-->");
        let y = x.eval_with_assign(state, env)?;
        println!("{y:#}");
        println!()
    }
    Ok(Expression::None)
}

// arg lazy
fn r#typeof(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("typeof", &args, 1, ctx)?;
    let t = args[0].type_name();
    let t1 = args[0].eval_with_assign(state, env)?.type_name();
    // println!("{}", t);
    // println!("----------");
    // println!("{}", t1);
    Ok(Expression::from(vec![
        Expression::from(t),
        Expression::from(t1),
    ]))
}

// Print Formated
fn printf(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("printf", &args, 1.., ctx)?;
    let template_expr = args[0].eval_mut(state, env, 0)?;
    let template = get_string_arg(template_expr, ctx)?;

    // named arg
    let pat = regex_lite::Regex::new(r#"\{(\w+)\}"#).unwrap();
    let mut result = template.clone();
    for (full, [var]) in pat.captures_iter(&template).map(|m| m.extract()) {
        let value = ctx.handle_variable(var, false, state, env, 0)?;
        result = result.replace(full, &value.to_string());
    }

    // position arg
    let placeholders = result.matches("{}").count();
    for arg in args.iter().skip(1).take(placeholders) {
        result = result.replacen("{}", &arg.eval_mut(state, env, 0)?.to_string(), 1);
    }

    println!("{}", result);
    Ok(Expression::None)
}

// lazy arg
pub fn set_root(
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("set_root", &args, 2, ctx)?;
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
    check_exact_args_len("unset_root", &args, 1, ctx)?;
    let name = args[0].to_string();
    env.undefine_in_root(&name);
    Ok(Expression::None)
}
