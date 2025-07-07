use crate::{Environment, Expression, Int, LmError};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
        // 数学常量（无参数）
        String::from("E")   => std::f64::consts::E.into(),
        String::from("PI")  => std::f64::consts::PI.into(),
        String::from("TAU") => std::f64::consts::TAU.into(),
        String::from("PHI") => 1.618033988749894848204586834365638118_f64.into(),

        // 基础数学函数
        String::from("max") => Expression::builtin("max", max, "get max value in an array or multi args", "<num1> <num2> ... | <array>"),
        String::from("min") => Expression::builtin("min", min, "get min value in an array or multi args", "<num1> <num2> ... | <array>"),
        String::from("sum") => Expression::builtin("sum", sum, "sum a list of numbers", "<num1> <num2> ... | <array>"),
        String::from("average") => Expression::builtin("average", average, "get the average of a list of numbers", "<num1> <num2> ... | <array>"),
        String::from("abs") => Expression::builtin("abs", abs, "get the absolute value of a number", "<number>"),
        String::from("clamp") => Expression::builtin("clamp", clamp, "clamp a value between min and max", "<min> <max> <value>"),

        // 位运算
        String::from("bit_and") => Expression::builtin("bit_and", bit_and, "bitwise AND operation", "<int1> <int2>"),
        String::from("bit_or") => Expression::builtin("bit_or", bit_or, "bitwise OR operation", "<int1> <int2>"),
        String::from("bit_xor") => Expression::builtin("bit_xor", bit_xor, "bitwise XOR operation", "<int1> <int2>"),
        String::from("bit_not") => Expression::builtin("bit_not", bit_not, "bitwise NOT operation", "<integer>"),
        String::from("bit_shl") => Expression::builtin("bit_shl", bit_shl, "bitwise shift left", "<shift_bits> <integer>"),
        String::from("bit_shr") => Expression::builtin("bit_shr", bit_shr, "bitwise shift right", "<shift_bits> <integer>"),

        // 三角函数（单位：弧度）
        String::from("sin") => Expression::builtin("sin", sin, "get the sine of a number", "<radians>"),
        String::from("cos") => Expression::builtin("cos", cos, "get the cosine of a number", "<radians>"),
        String::from("tan") => Expression::builtin("tan", tan, "get the tangent of a number", "<radians>"),
        String::from("asin") => Expression::builtin("asin", asin, "get the inverse sine of a number", "<value>"),
        String::from("acos") => Expression::builtin("acos", acos, "get the inverse cosine of a number", "<value>"),
        String::from("atan") => Expression::builtin("atan", atan, "get the inverse tangent of a number", "<value>"),

        // 双曲函数
        String::from("sinh") => Expression::builtin("sinh", sinh, "get the hyperbolic sine of a number", "<value>"),
        String::from("cosh") => Expression::builtin("cosh", cosh, "get the hyperbolic cosine of a number", "<value>"),
        String::from("tanh") => Expression::builtin("tanh", tanh, "get the hyperbolic tangent of a number", "<value>"),
        String::from("asinh") => Expression::builtin("asinh", asinh, "get the inverse hyperbolic sine of a number", "<value>"),
        String::from("acosh") => Expression::builtin("acosh", acosh, "get the inverse hyperbolic cosine of a number", "<value>"),
        String::from("atanh") => Expression::builtin("atanh", atanh, "get the inverse hyperbolic tangent of a number", "<value>"),

        // π倍三角函数
        String::from("sinpi") => Expression::builtin("sinpi", sinpi, "get the sine of a number times π", "<value>"),
        String::from("cospi") => Expression::builtin("cospi", cospi, "get the cosine of a number times π", "<value>"),
        String::from("tanpi") => Expression::builtin("tanpi", tanpi, "get the tangent of a number times π", "<value>"),

        // 指数与对数
        String::from("pow") => Expression::builtin("pow", pow, "raise a number to a power", "<exponent> <base>"),
        String::from("exp") => Expression::builtin("exp", exp, "get e raised to the power of a number", "<exponent>"),
        String::from("exp2") => Expression::builtin("exp2", exp2, "get 2 raised to the power of a number", "<exponent>"),
        String::from("sqrt") => Expression::builtin("sqrt", sqrt, "get the square root of a number", "<number>"),
        String::from("cbrt") => Expression::builtin("cbrt", cbrt, "get the cube root of a number", "<number>"),
        String::from("log") => Expression::builtin("log", log, "get the log of a number using a given base", "<number> <base>"),
        String::from("log2") => Expression::builtin("log2", log2, "get the log base 2 of a number", "<number>"),
        String::from("log10") => Expression::builtin("log10", log10, "get the log base 10 of a number", "<number>"),
        String::from("ln") => Expression::builtin("ln", ln, "natural logarithm", "<number>"),

        // 舍入函数
        String::from("floor") => Expression::builtin("floor", floor, "get the floor of a number", "<number>"),
        String::from("ceil") => Expression::builtin("ceil", ceil, "get the ceiling of a number", "<number>"),
        String::from("round") => Expression::builtin("round", round, "round a number to the nearest integer", "<number>"),
        String::from("trunc") => Expression::builtin("trunc", trunc, "truncate a number", "<number>"),

        // 其他函数
        String::from("isodd") => Expression::builtin("isodd", isodd, "is a number odd?", "<integer>"),
    }).into()
}

// Helper function to evaluate arguments to f64
fn eval_to_f64(
    args: &[Expression],
    env: &mut Environment,
    func_name: &str,
) -> Result<Vec<f64>, LmError> {
    args.iter()
        .map(|arg| match arg.eval(env)? {
            Expression::Integer(i) => Ok(i as f64),
            Expression::Float(f) => Ok(f),
            e => Err(LmError::CustomError(format!(
                "invalid {} argument {}",
                func_name, e
            ))),
        })
        .collect()
}

pub fn max(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    let nums = args_collect_iter(args, env)?;
    let mut max_val_int: Option<i64> = None; // 用于存储整数最大值
    let mut max_val_float: Option<f64> = None; // 用于存储浮点数最大值

    for num in nums {
        match num {
            Expression::Integer(i) => {
                if let Some(current_max) = max_val_int {
                    max_val_int = Some(current_max.max(i));
                } else {
                    max_val_int = Some(i);
                }
                // 由于我们遇到了整数，继续检查
            }
            Expression::Float(f) => {
                if let Some(current_max) = max_val_float {
                    max_val_float = Some(current_max.max(f));
                } else {
                    max_val_float = Some(f);
                }
            }
            _ => {
                return Err(LmError::CustomError(
                    "max requires numeric arguments".into(),
                ));
            }
        }
    }

    // 根据输入类型返回结果

    match max_val_float {
        Some(m_float) => match max_val_int {
            Some(m) => Ok(Expression::Float(m_float.max(m as f64))),
            None => Ok(Expression::Float(m_float)),
        },

        None => match max_val_int {
            Some(m) => Ok(Expression::Integer(m)),
            None => Ok(Expression::None),
        },
    }
}

// Optimized min function that handles both integers and floats
pub fn min(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    let nums = args_collect_iter(args, env)?;
    let mut min_val: Option<f64> = None;

    for num in nums {
        match num {
            Expression::Integer(i) => {
                if let Some(current_min) = min_val {
                    min_val = Some(current_min.min(i as f64));
                } else {
                    min_val = Some(i as f64);
                }
            }
            Expression::Float(f) => {
                if let Some(current_min) = min_val {
                    min_val = Some(current_min.min(f));
                } else {
                    min_val = Some(f);
                }
            }
            _ => {
                return Err(LmError::CustomError(
                    "min requires numeric arguments".into(),
                ));
            }
        }
    }

    match min_val {
        Some(m) => Ok(Expression::Float(m)),
        None => Ok(Expression::None),
    }
}

// Clamp function to limit a value between min and max
fn clamp(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("clamp", args, 3)?;

    let value = match args[2].eval(env)? {
        Expression::Integer(i) => i as f64,
        Expression::Float(f) => f,
        e => return Err(LmError::CustomError(format!("invalid clamp value {}", e))),
    };

    let min_val = match args[0].eval(env)? {
        Expression::Integer(i) => i as f64,
        Expression::Float(f) => f,
        e => return Err(LmError::CustomError(format!("invalid clamp min {}", e))),
    };

    let max_val = match args[1].eval(env)? {
        Expression::Integer(i) => i as f64,
        Expression::Float(f) => f,
        e => return Err(LmError::CustomError(format!("invalid clamp max {}", e))),
    };

    if min_val > max_val {
        return Err(LmError::CustomError("clamp min must be <= max".into()));
    }

    let result = if value < min_val {
        min_val
    } else if value > max_val {
        max_val
    } else {
        value
    };

    Ok(Expression::Float(result))
}

// Bitwise AND operation
fn bit_and(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("bit_and", args, 2)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_and argument {}",
                e
            )));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_and argument {}",
                e
            )));
        }
    };

    Ok(Expression::Integer(a & b))
}

// Bitwise OR operation
fn bit_or(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("bit_or", args, 2)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_or argument {}",
                e
            )));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_or argument {}",
                e
            )));
        }
    };

    Ok(Expression::Integer(a | b))
}

// Bitwise XOR operation
fn bit_xor(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("bit_xor", args, 2)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_xor argument {}",
                e
            )));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_xor argument {}",
                e
            )));
        }
    };

    Ok(Expression::Integer(a ^ b))
}

// Bitwise NOT operation
fn bit_not(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("bit_not", args, 1)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_not argument {}",
                e
            )));
        }
    };

    Ok(Expression::Integer(!a))
}

// Bitwise shift left
fn bit_shl(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("bit_shl", args, 2)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_shl shift_bit argument {}",
                e
            )));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_shl base argument {}",
                e
            )));
        }
    };

    Ok(Expression::Integer(b << a))
}

// Bitwise shift right
fn bit_shr(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("bit_shr", args, 2)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_shr shift_bit argument {}",
                e
            )));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(LmError::CustomError(format!(
                "invalid bit_shr base argument {}",
                e
            )));
        }
    };

    Ok(Expression::Integer(b >> a))
}

// Helper function to collect arguments (used by max/min)
fn args_collect_iter(
    args: &Vec<Expression>,
    env: &mut Environment,
) -> Result<Vec<Expression>, LmError> {
    match args.len() {
        2.. => Ok(args
            .iter()
            .map(|f| f.eval(env))
            .collect::<Result<Vec<_>, _>>()?),
        1 => match args[0].eval(env)? {
            Expression::List(li) => Ok(li.as_ref().clone()),
            _ => Err(LmError::CustomError(
                "the only arg for math.max/math.min should be a list".into(),
            )),
        },
        0 => Err(LmError::CustomError(
            "math.max/math.min requires 1 list or 2 or more nums".into(),
        )),
    }
}

// Implementations of all other math functions (kept similar to original but extracted)
fn abs(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("abs", args, 1)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.abs().into()),
        Expression::Float(f) => Ok(f.abs().into()),
        e => Err(LmError::CustomError(format!(
            "invalid abs argument {:?}",
            e
        ))),
    }
}

// Sum function that handles both integers and floats
pub fn sum(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    let nums = args_collect_iter(args, env)?;
    let mut int_sum = 0;
    let mut float_sum = 0.0;
    let mut has_float = false;

    for num in nums {
        match num {
            Expression::Integer(i) => {
                if has_float {
                    float_sum += i as f64;
                } else {
                    int_sum += i;
                }
            }
            Expression::Float(f) => {
                if !has_float {
                    float_sum = int_sum as f64;
                    has_float = true;
                }
                float_sum += f;
            }
            _ => {
                return Err(LmError::CustomError(
                    "sum requires numeric arguments".into(),
                ));
            }
        }
    }

    if has_float {
        Ok(Expression::Float(float_sum))
    } else {
        Ok(Expression::Integer(int_sum))
    }
}

// Average function that handles both integers and floats
pub fn average(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    let nums = args_collect_iter(args, env)?;
    if nums.is_empty() {
        return Err(LmError::CustomError(
            "average requires at least one number".into(),
        ));
    }

    let mut sum = 0.0;
    let mut count = 0;

    for num in nums {
        match num {
            Expression::Integer(i) => {
                sum += i as f64;
                count += 1;
            }
            Expression::Float(f) => {
                sum += f;
                count += 1;
            }
            _ => {
                return Err(LmError::CustomError(
                    "average requires numeric arguments".into(),
                ));
            }
        }
    }

    Ok(Expression::Float(sum / count as f64))
}

// Floor function
fn floor(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("floor", args, 1)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.into()),
        Expression::Float(f) => Ok(f.floor().into()),
        e => Err(LmError::CustomError(format!(
            "invalid floor argument {:?}",
            e
        ))),
    }
}

// Ceil function
fn ceil(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("ceil", args, 1)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.into()),
        Expression::Float(f) => Ok(f.ceil().into()),
        e => Err(LmError::CustomError(format!(
            "invalid ceil argument {:?}",
            e
        ))),
    }
}

// Round function
fn round(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("round", args, 1)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.into()),
        Expression::Float(f) => Ok(f.round().into()),
        e => Err(LmError::CustomError(format!(
            "invalid round argument {:?}",
            e
        ))),
    }
}

// Trunc function
fn trunc(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("trunc", args, 1)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.into()),
        Expression::Float(f) => Ok(f.trunc().into()),
        e => Err(LmError::CustomError(format!(
            "invalid trunc argument {:?}",
            e
        ))),
    }
}

// Isodd function
fn isodd(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("isodd", args, 1)?;
    Ok(match args[0].eval(env)? {
        Expression::Integer(i) => (i % 2 != 0).into(),
        Expression::Float(f) => ((f as Int) % 2 != 0).into(),
        e => {
            return Err(LmError::CustomError(format!(
                "invalid isodd argument {}",
                e
            )));
        }
    })
}

// Sqrt function
fn sqrt(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("sqrt", args, 1)?;
    let x = eval_to_f64(args, env, "sqrt")?[0];
    Ok(x.sqrt().into())
}

// Cbrt function
fn cbrt(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cbrt", args, 1)?;
    let x = eval_to_f64(args, env, "cbrt")?[0];
    Ok(x.cbrt().into())
}

// Exp function
fn exp(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("exp", args, 1)?;
    let x = eval_to_f64(args, env, "exp")?[0];
    Ok(x.exp().into())
}

// Exp2 function
fn exp2(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("exp2", args, 1)?;
    let x = eval_to_f64(args, env, "exp2")?[0];
    Ok(x.exp2().into())
}

// Log function
fn log(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("log", args, 2)?;
    let base = eval_to_f64(&args[0..1], env, "log")?[0];
    let x = eval_to_f64(&args[1..2], env, "log")?[0];
    Ok(x.log(base).into())
}

// Log2 function
fn log2(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("log2", args, 1)?;
    let x = eval_to_f64(args, env, "log2")?[0];
    Ok(x.log2().into())
}

// Log10 function
fn log10(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("log10", args, 1)?;
    let x = eval_to_f64(args, env, "log10")?[0];
    Ok(x.log10().into())
}

// Ln function
fn ln(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("ln", args, 1)?;
    let x = eval_to_f64(args, env, "ln")?[0];
    Ok(x.ln().into())
}

// 幂函数
fn pow(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("pow", args, 2)?;
    let nums = eval_to_f64(args, env, "pow")?;
    Ok(nums[1].powf(nums[0]).into())
}

// 正弦函数
fn sin(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("sin", args, 1)?;
    let x = eval_to_f64(args, env, "sin")?[0];
    Ok(x.sin().into())
}

// 余弦函数
fn cos(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cos", args, 1)?;
    let x = eval_to_f64(args, env, "cos")?[0];
    Ok(x.cos().into())
}

// 正切函数
fn tan(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("tan", args, 1)?;
    let x = eval_to_f64(args, env, "tan")?[0];
    Ok(x.tan().into())
}

// 反余弦函数
fn acos(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("acos", args, 1)?;
    let x = eval_to_f64(args, env, "acos")?[0];
    Ok(x.acos().into())
}

// 反正弦函数
fn asin(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("asin", args, 1)?;
    let x = eval_to_f64(args, env, "asin")?[0];
    Ok(x.asin().into())
}

// 反正切函数
fn atan(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("atan", args, 1)?;
    let x = eval_to_f64(args, env, "atan")?[0];
    Ok(x.atan().into())
}

// 双曲余弦函数
fn cosh(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cosh", args, 1)?;
    let x = eval_to_f64(args, env, "cosh")?[0];
    Ok(x.cosh().into())
}

// 双曲正弦函数
fn sinh(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("sinh", args, 1)?;
    let x = eval_to_f64(args, env, "sinh")?[0];
    Ok(x.sinh().into())
}

// 双曲正切函数
fn tanh(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("tanh", args, 1)?;
    let x = eval_to_f64(args, env, "tanh")?[0];
    Ok(x.tanh().into())
}

// 反双曲余弦函数
fn acosh(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("acosh", args, 1)?;
    let x = eval_to_f64(args, env, "acosh")?[0];
    Ok(x.acosh().into())
}

// 反双曲正弦函数
fn asinh(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("asinh", args, 1)?;
    let x = eval_to_f64(args, env, "asinh")?[0];
    Ok(x.asinh().into())
}

// 反双曲正切函数
fn atanh(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("atanh", args, 1)?;
    let x = eval_to_f64(args, env, "atanh")?[0];
    Ok(x.atanh().into())
}

// π倍三角函数
fn sinpi(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("sinpi", args, 1)?;
    let x = eval_to_f64(args, env, "sinpi")?[0];
    Ok((x * std::f64::consts::PI).sin().into())
}

fn cospi(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("cospi", args, 1)?;
    let x = eval_to_f64(args, env, "cospi")?[0];
    Ok((x * std::f64::consts::PI).cos().into())
}

fn tanpi(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("tanpi", args, 1)?;
    let x = eval_to_f64(args, env, "tanpi")?[0];
    Ok((x * std::f64::consts::PI).tan().into())
}
