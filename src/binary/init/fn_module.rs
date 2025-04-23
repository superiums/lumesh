use super::{Environment, LmError, Expression};
use common_macros::b_tree_map;
use lumesh::parse;

// 柯里化函数构建（适配多参数Lambda）
pub(super) fn curry_env(
    f: Expression,
    args: usize,
    env: &mut Environment,
) -> Result<Expression, LmError> {
    // 生成参数列表 (arg0, arg1, ...)
    let params: Vec<String> = (0..args).map(|i| format!("arg{}", i)).collect();

    // 构建参数应用表达式
    let mut applied = Expression::Apply(
        Box::new(f.clone()),
        params
            .iter()
            .map(|name| Expression::Symbol(name.clone()))
            .collect(),
    );

    // 递归构建嵌套Lambda
    for (i, param) in params.iter().enumerate().rev() {
        applied = Expression::Lambda(
            vec![param.clone()], // 单参数形式
            Box::new(applied),
            env.fork(), // 捕获当前环境
        );
    }

    Ok(applied)
}

// 反向柯里化（适配新参数结构）
pub(super) fn reverse_curry_env(
    f: Expression,
    args: usize,
    env: &mut Environment,
) -> Result<Expression, LmError> {
    // 生成反向参数列表
    let params: Vec<String> = (0..args).rev().map(|i| format!("arg{}", i)).collect();

    // 构建应用表达式
    let mut applied = Expression::Apply(
        Box::new(f.clone()),
        params
            .iter()
            .map(|name| Expression::Symbol(name.clone()))
            .collect(),
    );

    // 反向嵌套Lambda
    for param in params.iter() {
        applied = Expression::Lambda(vec![param.clone()], Box::new(applied), env.fork());
    }

    Ok(applied)
}

// 核心高阶函数定义
pub fn get() -> Expression {
    let mut env = Environment::new();

    // 使用新Lambda语法定义
    let id = parse_lambda("(x) -> x", &mut env);
    let const_fn = parse_lambda("(x, y) -> x", &mut env);
    let flip = parse_lambda("(f, x, y) -> f y x", &mut env);
    let compose = parse_lambda("(f, g, x) -> f (g x)", &mut env);

    // 内置函数模块
    Expression::Map(b_tree_map! {
        String::from("id") => id,
            String::from("const") => const_fn,
            String::from("flip") => flip,
            String::from("compose") => compose,
            String::from("apply") => build_apply(),
            String::from("curry") => build_curry(),

        String::from("map") => Expression::builtin("map", map,
            "map a function over a list of values"),
        String::from("filter") => Expression::builtin("filter", filter,
            "filter a list of values with a condition function"),
        String::from("reduce") => Expression::builtin("reduce", reduce,
            "reduce a function over a list of values"),
        String::from("?") => Expression::builtin("?", conditional,
        "conditionally evaluate two expressions based on the truthiness of a condition"),

    })
}

// Lambda解析辅助函数
fn parse_lambda(code: &str, env: &mut Environment) -> Expression {
    let expr = parse(code)
        .expect("Parse failed")
        .eval(env)
        .expect("Eval failed");
    expr
}

// 构建apply函数（适配多参数）
fn build_apply() -> Expression {
    Expression::builtin(
        "apply",
        |args, env| {
            if args.len() != 2 {
                return Err(LmError::ArgumentMismatch {
                    name: args[0].to_string(),
                    expected: 2,
                    received: args.len(),
                });
            }

            let f = args[0].eval(env)?;
            let args_list = match args[1].eval(env)? {
                Expression::List(v) => v,
                _ => {
                    return Err(LmError::TypeError {
                        expected: "list".into(),
                        found: args[1].type_name(),
                    });
                }
            };

            Ok(Expression::Apply(Box::new(f), args_list))
        },
        "Apply function to argument list",
    )
}

// 构建curry函数（适配多参数Lambda）
fn build_curry() -> Expression {
    Expression::builtin(
        "curry",
        |args, env| {
            if args.len() < 2 {
                return Err(LmError::ArgumentMismatch {
                    name: args[0].to_string(),
                    expected: 2,
                    received: args.len(),
                });
            }

            let f = args[0].eval(env)?;
            let arg_count = match args[1].eval(env)? {
                Expression::Integer(n) => n as usize,
                _ => {
                    return Err(LmError::TypeError {
                        expected: "integer".into(),
                        found: args[1].type_name(),
                    });
                }
            };

            curry_env(f, arg_count, env)
        },
        "Curry a multi-argument function",
    )
}

// 其他辅助宏和函数保持不变...

pub fn reverse_curry(f: Expression, args: usize) -> Expression {
    let mut env = Environment::default();
    reverse_curry_env(f, args, &mut env).unwrap()
}

pub(super) fn curry(f: Expression, args: usize) -> Expression {
    let mut env = Environment::default();
    curry_env(f, args, &mut env).unwrap()
}

fn curry_builtin(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() < 2 {
        return Err(LmError::CustomError(
            "curry requires at least two arguments".to_string(),
        ));
    }
    let f = args[0].eval(env)?;
    if let Expression::Integer(arg_count) = args[1].eval(env)? {
        curry_env(f, arg_count as usize, env)
    } else {
        Ok(f)
    }
}

fn conditional(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if args.len() != 3 {
        return Err(LmError::CustomError(
            "conditional requires exactly three arguments".to_string(),
        ));
    }
    let condition = args[0].eval(env)?;
    if condition.is_truthy() {
        args[1].eval(env)
    } else {
        args[2].eval(env)
    }
}

fn map(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if !(1..=2).contains(&args.len()) {
        return Err(LmError::CustomError(
            if args.len() > 2 {
                "too many arguments to function map"
            } else {
                "too few arguments to function map"
            }
            .to_string(),
        ));
    }

    if args.len() == 1 {
        Expression::Apply(
            Box::new(lumesh::parse("(f,list) -> for item in list {f item}")?),
            args.clone(),
        )
        .eval(env)
    } else if let Expression::List(list) = args[1].eval(env)? {
        let f = args[0].eval(env)?;
        let mut result = vec![];
        for item in list {
            result.push(Expression::Apply(Box::new(f.clone()), vec![item]).eval(env)?)
        }
        Ok(result.into())
    } else {
        Err(LmError::CustomError(format!(
            "invalid arguments to map: {}",
            Expression::from(args)
        )))
    }
}

fn filter(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if !(1..=2).contains(&args.len()) {
        return Err(LmError::CustomError(
            if args.len() > 2 {
                "too many arguments to function filter"
            } else {
                "too few arguments to function filter"
            }
            .to_string(),
        ));
    }

    if args.len() == 1 {
        Expression::Apply(
            Box::new(lumesh::parse(
                r#"(f,list) -> {
                    let result = [];
                    for item in list {
                        if (f item) {
                            let result = result + [item];
                        }
                    }
                    result
                }"#,
            )?),
            args.clone(),
        )
        .eval(env)
    } else if let Expression::List(list) = args[1].eval(env)? {
        let f = args[0].eval(env)?;
        let mut result = vec![];
        for item in list {
            if Expression::Apply(Box::new(f.clone()), vec![item.clone()])
                .eval(env)?
                .is_truthy()
            {
                result.push(item)
            }
        }
        Ok(result.into())
    } else {
        Err(LmError::CustomError(format!(
            "invalid arguments to filter: {}",
            Expression::from(args)
        )))
    }
}

fn reduce(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    if !(1..=3).contains(&args.len()) {
        return Err(LmError::CustomError(
            if args.len() > 3 {
                "too many arguments to function reduce"
            } else {
                "too few arguments to function reduce"
            }
            .to_string(),
        ));
    }

    if args.len() < 3 {
        Expression::Apply(
            Box::new(lumesh::parse(
                "(f,acc,list) -> { \
                        for item in list { let acc = f acc item } acc }",
            )?),
            args.clone(),
        )
        .eval(env)
    } else if let Expression::List(list) = args[2].eval(env)? {
        let f = args[0].eval(env)?;
        let mut acc = args[1].eval(env)?;
        for item in list {
            acc = Expression::Apply(Box::new(f.clone()), vec![acc, item]).eval(env)?
        }
        Ok(acc)
    } else {
        Err(LmError::CustomError(format!(
            "invalid arguments to reduce: {}",
            Expression::from(args)
        )))
    }
}
