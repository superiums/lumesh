use std::collections::HashMap;

use crate::libs::BuiltinInfo;
use crate::libs::helper::{check_args_len, check_exact_args_len, get_string_arg};
use crate::libs::lazy_module::LazyModule;
use crate::{Environment, Expression, Int, RuntimeError, RuntimeErrorKind, reg_info, reg_lazy};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // 数学常量（无参数）
         // E  , PI , TAU, PHI,
         // 基础数学函数
         max, min, sum, average, abs, clamp,
         // 位运算
         bit_and, bit_or, bit_xor, bit_not, bit_shl, bit_shr,
         //逻辑运算
         gt, ge, lt, le, eq, ne,
         // 三角函数（单位：弧度）
         sin, cos, tan, asin, acos, atan,
         // 双曲函数
         sinh, cosh, tanh, asinh, acosh, atanh,
         // π倍三角函数
         sinpi, cospi, tanpi,
         // 指数与对数
         pow, exp, exp2, sqrt, cbrt, log, log2, log10, ln,
         // 舍入函数
         floor, ceil, round, trunc,
         // 其他函数
         isodd,
         // to_str,
    })
}
pub fn regist_info() -> HashMap<&'static str, BuiltinInfo> {
    reg_info!({

        // 数学常量（无参数）
         // E   => std::f64::consts::E.into(),
         // PI  => std::f64::consts::PI.into(),
         // TAU => std::f64::consts::TAU.into(),
         // PHI => 1.618_033_988_749_895_f64.into(),

         // 基础数学函数
         max => "get max value in an array or multi args", "<num1> <num2> ... | <array>"
         min => "get min value in an array or multi args", "<num1> <num2> ... | <array>"
         sum => "sum a list of numbers", "<num1> <num2> ... | <array>"
         average => "get the average of a list of numbers", "<num1> <num2> ... | <array>"
         abs => "get the absolute value of a number", "<number>"
         clamp => "clamp a value between min and max", "<min> <max> <value>"

         // 位运算
         bit_and => "bitwise AND operation", "<int1> <int2>"
         bit_or => "bitwise OR operation", "<int1> <int2>"
         bit_xor => "bitwise XOR operation", "<int1> <int2>"
         bit_not => "bitwise NOT operation", "<integer>"
         bit_shl => "bitwise shift left", "<shift_bits> <integer>"
         bit_shr => "bitwise shift right", "<shift_bits> <integer>"

         //逻辑运算
         gt => "check if greater than", "<number> <number_base>"
         ge => "check if greater than or equal", "<number> <number_base>"
         lt => "check if lower than", "<number> <number_base>"
         le => "check if lower than or equal", "<number> <number_base>"
         eq => "check if equal", "<number> <number_base>"
         ne => "check if NOT equal", "<number> <number_base>"

         // 三角函数（单位：弧度）
         sin => "get the sine of a number", "<radians>"
         cos => "get the cosine of a number", "<radians>"
         tan => "get the tangent of a number", "<radians>"
         asin => "get the inverse sine of a number", "<value>"
         acos => "get the inverse cosine of a number", "<value>"
         atan => "get the inverse tangent of a number", "<value>"

         // 双曲函数
         sinh => "get the hyperbolic sine of a number", "<value>"
         cosh => "get the hyperbolic cosine of a number", "<value>"
         tanh => "get the hyperbolic tangent of a number", "<value>"
         asinh => "get the inverse hyperbolic sine of a number", "<value>"
         acosh => "get the inverse hyperbolic cosine of a number", "<value>"
         atanh => "get the inverse hyperbolic tangent of a number", "<value>"

         // π倍三角函数
         sinpi => "get the sine of a number times π", "<value>"
         cospi => "get the cosine of a number times π", "<value>"
         tanpi => "get the tangent of a number times π", "<value>"

         // 指数与对数
         pow => "raise a number to a power", "<exponent> <base>"
         exp => "get e raised to the power of a number", "<exponent>"
         exp2 => "get 2 raised to the power of a number", "<exponent>"
         sqrt => "get the square root of a number", "<number>"
         cbrt => "get the cube root of a number", "<number>"
         log => "get the log of a number using a given base", "<number> <base>"
         log2 => "get the log base 2 of a number", "<number>"
         log10 => "get the log base 10 of a number", "<number>"
         ln => "natural logarithm", "<number>"

         // 舍入函数
         floor => "get the floor of a number", "<number>"
         ceil => "get the ceiling of a number", "<number>"
         round => "round a number to the nearest integer", "<number>"
         trunc => "truncate a number", "<number>"

         // 其他函数
         isodd => "is a number odd?", "<integer>"
         to_str => "trans to String", "<number>"


    })
}

// Helper Functions
// Helper function to evaluate arguments to f64
fn eval_to_f64(
    args: &[Expression],
    env: &mut Environment,
    func_name: &str,
    ctx: &Expression,
) -> Result<Vec<f64>, RuntimeError> {
    args.iter()
        .map(|arg| match arg.eval(env)? {
            Expression::Integer(i) => Ok(i as f64),
            Expression::Float(f) => Ok(f),
            e => Err(RuntimeError::common(
                format!("invalid {func_name} argument {e}").into(),
                ctx.clone(),
                0,
            )),
        })
        .collect()
}

// Helper function to collect arguments (used by max/min)
fn args_collect_iter(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Vec<Expression>, RuntimeError> {
    match args.len() {
        2.. => Ok(args
            .iter()
            .map(|f| f.eval(env))
            .collect::<Result<Vec<_>, _>>()?),
        1 => match args[0].eval(env)? {
            Expression::List(li) => Ok(li.as_ref().clone()),
            Expression::Range(r, step) => {
                Ok(r.step_by(step).map(Expression::Integer).collect::<Vec<_>>())
            }
            _ => Err(RuntimeError::common(
                "the only arg for math.max/math.min should be a list".into(),
                ctx.clone(),
                0,
            )),
        },
        0 => Err(RuntimeError::common(
            "math.max/math.min requires 1 list or multi nums".into(),
            ctx.clone(),
            0,
        )),
    }
}

pub fn get_float_arg(expr: Expression, ctx: &Expression) -> Result<f64, RuntimeError> {
    match expr {
        Expression::Integer(i) => Ok(i as f64),
        Expression::Float(i) => Ok(i),
        e => Err(RuntimeError::new(
            RuntimeErrorKind::TypeError {
                expected: "Integer/Float".to_string(),
                found: e.type_name(),
                sym: e.to_string(),
            },
            ctx.clone(),
            0,
        )),
    }
}
// Basic Math Functions
pub fn max(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let nums = args_collect_iter(args, env, ctx)?;
    let mut max_val_int: Option<i64> = None;
    let mut max_val_float: Option<f64> = None;

    for num in nums {
        match num {
            Expression::Integer(i) => {
                if let Some(current_max) = max_val_int {
                    max_val_int = Some(current_max.max(i));
                } else {
                    max_val_int = Some(i);
                }
            }
            Expression::Float(f) => {
                if let Some(current_max) = max_val_float {
                    max_val_float = Some(current_max.max(f));
                } else {
                    max_val_float = Some(f);
                }
            }
            _ => {
                return Err(RuntimeError::common(
                    "max requires numeric arguments".into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    }

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

pub fn min(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let nums = args_collect_iter(args, env, ctx)?;
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
                return Err(RuntimeError::common(
                    "min requires numeric arguments".into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    }

    match min_val {
        Some(m) => Ok(Expression::Float(m)),
        None => Ok(Expression::None),
    }
}

fn clamp(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("clamp", args, 3, ctx)?;

    let value = match args[0].eval(env)? {
        Expression::Integer(i) => i as f64,
        Expression::Float(f) => f,
        e => {
            return Err(RuntimeError::common(
                format!("invalid clamp value {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let min_val = match args[1].eval(env)? {
        Expression::Integer(i) => i as f64,
        Expression::Float(f) => f,
        e => {
            return Err(RuntimeError::common(
                format!("invalid clamp min {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let max_val = match args[2].eval(env)? {
        Expression::Integer(i) => i as f64,
        Expression::Float(f) => f,
        e => {
            return Err(RuntimeError::common(
                format!("invalid clamp max {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    if min_val > max_val {
        return Err(RuntimeError::common(
            "clamp min must be <= max".into(),
            ctx.clone(),
            0,
        ));
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
// Bitwise Operations
fn bit_and(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("bit_and", args, 2, ctx)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_and argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_and argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    Ok(Expression::Integer(a & b))
}

fn bit_or(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("bit_or", args, 2, ctx)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_or argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_or argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    Ok(Expression::Integer(a | b))
}

fn bit_xor(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("bit_xor", args, 2, ctx)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_xor argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_xor argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    Ok(Expression::Integer(a ^ b))
}

fn bit_not(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("bit_not", args, 1, ctx)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_not argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    Ok(Expression::Integer(!a))
}

fn bit_shl(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("bit_shl", args, 2, ctx)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_shl base argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_shl shift_bit argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };
    if b < 0 || b > 63 {
        return Err(RuntimeError::common(
            format!("shift amount {} out of range (0-63)", a).into(),
            ctx.clone(),
            0,
        ));
    }
    Ok(Expression::Integer(a << b))
}

fn bit_shr(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("bit_shr", args, 2, ctx)?;

    let a = match args[0].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_shr base argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let b = match args[1].eval(env)? {
        Expression::Integer(i) => i,
        e => {
            return Err(RuntimeError::common(
                format!("invalid bit_shr shift_bit argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    };
    if b < 0 || b > 63 {
        return Err(RuntimeError::common(
            format!("shift amount {} out of range (0-63)", a).into(),
            ctx.clone(),
            0,
        ));
    }
    Ok(Expression::Integer(a >> b))
}
// Comparison Functions
fn gt(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("gt", args, 2, ctx)?;
    let base = get_float_arg(args[0].eval(env)?, ctx)?;
    let other = get_float_arg(args[1].eval(env)?, ctx)?;
    Ok(Expression::Boolean(base.gt(&other)))
}

fn ge(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("ge", args, 2, ctx)?;
    let base = get_float_arg(args[0].eval(env)?, ctx)?;
    let other = get_float_arg(args[1].eval(env)?, ctx)?;
    Ok(Expression::Boolean(base.ge(&other)))
}

fn lt(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("lt", args, 2, ctx)?;
    let base = get_float_arg(args[0].eval(env)?, ctx)?;
    let other = get_float_arg(args[1].eval(env)?, ctx)?;
    Ok(Expression::Boolean(base.lt(&other)))
}

fn le(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("le", args, 2, ctx)?;
    let base = get_float_arg(args[0].eval(env)?, ctx)?;
    let other = get_float_arg(args[1].eval(env)?, ctx)?;
    Ok(Expression::Boolean(base.le(&other)))
}

fn eq(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("eq", args, 2, ctx)?;
    let base = get_float_arg(args[0].eval(env)?, ctx)?;
    let other = get_float_arg(args[1].eval(env)?, ctx)?;
    Ok(Expression::Boolean(base.eq(&other)))
}

fn ne(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("ne", args, 2, ctx)?;
    let base = get_float_arg(args[0].eval(env)?, ctx)?;
    let other = get_float_arg(args[1].eval(env)?, ctx)?;
    Ok(Expression::Boolean(base.ne(&other)))
}
// Basic Math Operations
fn abs(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("abs", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.abs().into()),
        Expression::Float(f) => Ok(f.abs().into()),
        e => Err(RuntimeError::common(
            format!("invalid abs argument {e:?}").into(),
            ctx.clone(),
            0,
        )),
    }
}

pub fn sum(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let nums = args_collect_iter(args, env, ctx)?;
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
                return Err(RuntimeError::common(
                    "sum requires numeric arguments".into(),
                    ctx.clone(),
                    0,
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

pub fn average(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let nums = args_collect_iter(args, env, ctx)?;
    if nums.is_empty() {
        return Err(RuntimeError::common(
            "average requires at least one number".into(),
            ctx.clone(),
            0,
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
                return Err(RuntimeError::common(
                    "average requires numeric arguments".into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    }

    Ok(Expression::Float(sum / count as f64))
}
// Rounding Functions
fn floor(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("floor", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.into()),
        Expression::Float(f) => Ok(f.floor().into()),
        e => Err(RuntimeError::common(
            format!("invalid floor argument {e:?}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn ceil(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("ceil", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.into()),
        Expression::Float(f) => Ok(f.ceil().into()),
        e => Err(RuntimeError::common(
            format!("invalid ceil argument {e:?}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn round(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("round", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.into()),
        Expression::Float(f) => Ok(f.round().into()),
        e => Err(RuntimeError::common(
            format!("invalid round argument {e:?}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn trunc(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("trunc", args, 1, ctx)?;
    match args[0].eval(env)? {
        Expression::Integer(i) => Ok(i.into()),
        Expression::Float(f) => Ok(f.trunc().into()),
        e => Err(RuntimeError::common(
            format!("invalid trunc argument {e:?}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn isodd(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("isodd", args, 1, ctx)?;
    Ok(match args[0].eval(env)? {
        Expression::Integer(i) => (i % 2 != 0).into(),
        Expression::Float(f) => ((f as Int) % 2 != 0).into(),
        e => {
            return Err(RuntimeError::common(
                format!("invalid isodd argument {e}").into(),
                ctx.clone(),
                0,
            ));
        }
    })
}
// Mathematical Functions
fn sqrt(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("sqrt", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "sqrt", ctx)?[0];
    Ok(x.sqrt().into())
}

fn cbrt(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cbrt", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "cbrt", ctx)?[0];
    Ok(x.cbrt().into())
}

fn exp(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("exp", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "exp", ctx)?[0];
    Ok(x.exp().into())
}

fn exp2(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("exp2", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "exp2", ctx)?[0];
    Ok(x.exp2().into())
}

fn log(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("log", args, 2, ctx)?;
    let base = eval_to_f64(&args[0..1], env, "log", ctx)?[0];
    let x = eval_to_f64(&args[1..2], env, "log", ctx)?[0];
    Ok(x.log(base).into())
}

fn log2(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("log2", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "log2", ctx)?[0];
    Ok(x.log2().into())
}

fn log10(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("log10", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "log10", ctx)?[0];
    Ok(x.log10().into())
}

fn ln(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("ln", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "ln", ctx)?[0];
    Ok(x.ln().into())
}

fn pow(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("pow", args, 2, ctx)?;
    let nums = eval_to_f64(args, env, "pow", ctx)?;
    Ok(nums[0].powf(nums[1]).into())
}
// Trigonometric Functions
fn sin(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("sin", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "sin", ctx)?[0];
    Ok(x.sin().into())
}

fn cos(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cos", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "cos", ctx)?[0];
    Ok(x.cos().into())
}

fn tan(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("tan", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "tan", ctx)?[0];
    Ok(x.tan().into())
}

fn asin(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("asin", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "asin", ctx)?[0];
    Ok(x.asin().into())
}

fn acos(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("acos", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "acos", ctx)?[0];
    Ok(x.acos().into())
}

fn atan(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("atan", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "atan", ctx)?[0];
    Ok(x.atan().into())
}
// Hyperbolic Functions
fn sinh(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("sinh", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "sinh", ctx)?[0];
    Ok(x.sinh().into())
}

fn cosh(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cosh", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "cosh", ctx)?[0];
    Ok(x.cosh().into())
}

fn tanh(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("tanh", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "tanh", ctx)?[0];
    Ok(x.tanh().into())
}

fn asinh(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("asinh", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "asinh", ctx)?[0];
    Ok(x.asinh().into())
}

fn acosh(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("acosh", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "acosh", ctx)?[0];
    Ok(x.acosh().into())
}

fn atanh(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("atanh", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "atanh", ctx)?[0];
    Ok(x.atanh().into())
}
// Pi Multiple Functions
fn sinpi(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("sinpi", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "sinpi", ctx)?[0];
    Ok((x * std::f64::consts::PI).sin().into())
}

fn cospi(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("cospi", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "cospi", ctx)?[0];
    Ok((x * std::f64::consts::PI).cos().into())
}

fn tanpi(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("tanpi", args, 1, ctx)?;
    let x = eval_to_f64(args, env, "tanpi", ctx)?[0];
    Ok((x * std::f64::consts::PI).tan().into())
}
