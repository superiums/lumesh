use crate::{Environment, Expression, LmError};
use chrono::{
    DateTime, Datelike, Duration as ChronoDuration, FixedOffset, Local, NaiveDate, NaiveDateTime,
    TimeZone, Timelike,
};
use common_macros::hash_map;
use std::{collections::HashMap, thread, time::Duration};

pub fn get() -> Expression {
    (hash_map! {
        // 基本时间获取函数
        String::from("sleep") => Expression::builtin("sleep", sleep,
            "sleep for a given number of milliseconds [ms] or duration string (e.g. '1s', '2m')"),

        String::from("display") => Expression::builtin("display", display,
            "get preformatted datetime as map with time/date/datetime/etc."),

        String::from("year") => Expression::builtin("year", |args, env| get_time_component(args, env, |dt| dt.year() as i64),
            "get year (current or from specified datetime)"),

        String::from("month") => Expression::builtin("month", |args, env| get_time_component(args, env, |dt| dt.month() as i64),
            "get month (1-12, current or from specified datetime)"),

        String::from("weekday") => Expression::builtin("weekday", |args, env| get_time_component(args, env, |dt| dt.weekday().num_days_from_monday() as i64 + 1),
            "get weekday (1-7, Monday=1, current or from specified datetime)"),

        String::from("day") => Expression::builtin("day", |args, env| get_time_component(args, env, |dt| dt.day() as i64),
            "get day of month (1-31, current or from specified datetime)"),

        String::from("hour") => Expression::builtin("hour", |args, env| get_time_component(args, env, |dt| dt.hour() as i64),
            "get hour (0-23, current or from specified datetime)"),

        String::from("minute") => Expression::builtin("minute", |args, env| get_time_component(args, env, |dt| dt.minute() as i64),
            "get minute (0-59, current or from specified datetime)"),

        String::from("second") => Expression::builtin("second", |args, env| get_time_component(args, env, |dt| dt.second() as i64),
            "get second (0-59, current or from specified datetime)"),

        String::from("seconds") => Expression::builtin("seconds", |args, env| get_time_component(args, env, |dt| dt.num_seconds_from_midnight() as i64),
            "get seconds since midnight (current or from specified datetime)"),

        String::from("stamp") => Expression::builtin("stamp", |args, env| get_time_component(args, env, |dt| dt.timestamp()),
            "get Unix timestamp in seconds (current or from specified datetime)"),

        String::from("stamp-ms") => Expression::builtin("stamp_ms", |args, env| get_time_component(args, env, |dt| dt.timestamp_millis()),
            "get Unix timestamp in milliseconds (current or from specified datetime)"),

        String::from("fmt") => Expression::builtin("fmt", fmt,
            "format datetime (current or specified) using chrono format string"),

        // 新增时间操作函数
        String::from("now") => Expression::builtin("now", now,
            "get current datetime as timestamp with optional format"),

        String::from("parse") => Expression::builtin("parse", parse,
            "parse datetime string according to format"),

        String::from("add_duration") => Expression::builtin("add_duration", add_duration,
            "add duration to datetime (duration string like '1h30m' or components)"),

        String::from("diff") => Expression::builtin("diff", diff,
            "calculate difference between two datetimes in specified units"),

        // String::from("timer") => Expression::builtin("timer", timer,
        //     "simple timer that executes a function after delay"),

        String::from("timezone") => Expression::builtin("timezone", timezone,
            "convert datetime to different timezone (offset in hours)"),

        String::from("is_leap_year") => Expression::builtin("is_leap_year", |args, env| {
            let year = match args.first().map(|a| a.eval(env)) {
                Some(Ok(Expression::Integer(y))) => y,
                Some(Ok(_)) => return Err(LmError::CustomError("Year must be an integer".to_string())),
                Some(Err(e)) => return Err(e.into()),
                None => Local::now().year() as i64,
            };

            Ok(Expression::Boolean(chrono::NaiveDate::from_ymd_opt(year as i32, 1, 1)
                .map(|d| d.leap_year())
                .unwrap_or(false)))
        }, "check if a year is a leap year")
    })
    .into()
}

// 实现辅助函数和主要函数

/// 获取时间组件（支持从指定时间或当前时间）
fn get_time_component<F>(
    args: &Vec<Expression>,
    env: &mut Environment,
    extractor: F,
) -> Result<Expression, LmError>
where
    F: Fn(DateTime<Local>) -> i64,
{
    match args.len() {
        0 => Ok(Expression::Integer(extractor(Local::now()))),
        1 => {
            let dt = parse_datetime_arg(&args[0], env)?;
            Ok(Expression::Integer(extractor(dt)))
        }
        _ => Err(LmError::CustomError(
            "Expected 0 or 1 arguments".to_string(),
        )),
    }
}

/// 解析日期时间参数（支持字符串、时间戳或字典格式）
fn parse_datetime_arg(arg: &Expression, env: &mut Environment) -> Result<DateTime<Local>, LmError> {
    match arg.eval(env)? {
        Expression::String(s) => {
            // 尝试解析常见格式
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return Ok(dt.with_timezone(&Local));
            }
            if let Ok(dt) = DateTime::parse_from_rfc2822(&s) {
                return Ok(dt.with_timezone(&Local));
            }
            if let Ok(ts) = s.parse::<i64>() {
                return Ok(Local.timestamp_opt(ts, 0).unwrap());
            }
            Err(LmError::CustomError(format!(
                "Unrecognized datetime format: {}",
                s
            )))
        }
        Expression::Integer(ts) => Ok(Local.timestamp_opt(ts, 0).unwrap()),
        Expression::Map(m) => {
            let map = m.as_ref();
            let year = get_map_value(map, "year")?.unwrap_or(Local::now().year() as i64);
            let month = get_map_value(map, "month")?.unwrap_or(1) as u32;
            let day = get_map_value(map, "day")?.unwrap_or(1) as u32;
            let hour = get_map_value(map, "hour")?.unwrap_or(0) as u32;
            let minute = get_map_value(map, "minute")?.unwrap_or(0) as u32;
            let second = get_map_value(map, "second")?.unwrap_or(0) as u32;

            NaiveDate::from_ymd_opt(year as i32, month, day)
                .and_then(|d| d.and_hms_opt(hour, minute, second))
                .map(|ndt| Local.from_local_datetime(&ndt).unwrap())
                .ok_or(LmError::CustomError("Invalid date components".to_string()))
        }
        _ => Err(LmError::CustomError(
            "Expected string, timestamp or map for datetime".to_string(),
        )),
    }
}

fn get_map_value(map: &HashMap<String, Expression>, key: &str) -> Result<Option<i64>, LmError> {
    match map.get(key) {
        Some(Expression::Integer(n)) => Ok(Some(*n)),
        Some(Expression::String(s)) => s
            .parse()
            .map(Some)
            .map_err(|_| LmError::CustomError(format!("Invalid integer value for {}", key))),
        None => Ok(None),
        _ => Err(LmError::CustomError(format!(
            "Expected integer for {}",
            key
        ))),
    }
}

// 主要功能函数实现

fn sleep(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("sleep", &args, 1)?;

    let duration = match args[0].eval(env)? {
        Expression::Float(n) if n > 0.0 => Duration::from_millis(n as u64),
        Expression::Integer(n) if n > 0 => Duration::from_millis(n as u64),
        Expression::String(s) => parse_duration_string(&s)?,
        otherwise => {
            return Err(LmError::CustomError(format!(
                "expected positive number or duration string, got {}",
                otherwise
            )));
        }
    };

    thread::sleep(duration);
    Ok(Expression::None)
}

/// 解析持续时间字符串如 "1h30m15s"
fn parse_duration_string(s: &str) -> Result<Duration, LmError> {
    let mut total_ms = 0u64;
    let mut num = 0u64;

    for c in s.chars() {
        if c.is_ascii_digit() {
            num = num * 10 + c.to_digit(10).unwrap() as u64;
        } else {
            let multiplier = match c.to_ascii_lowercase() {
                's' => 1_000,
                'm' => 60 * 1_000,
                'h' => 60 * 60 * 1_000,
                'd' => 24 * 60 * 60 * 1_000,
                _ => {
                    return Err(LmError::CustomError(format!(
                        "Unknown duration unit: {}",
                        c
                    )));
                }
            };
            total_ms += num * multiplier;
            num = 0;
        }
    }

    // 处理不带单位的情况，默认毫秒
    if num > 0 {
        total_ms += num;
    }

    Ok(Duration::from_millis(total_ms))
}

fn display(_: &Vec<Expression>, _: &mut Environment) -> Result<Expression, LmError> {
    let now = Local::now();
    Ok(Expression::from(hash_map! {
        String::from("time") => Expression::String(now.time().format("%H:%M:%S").to_string()),
        String::from("timepm") => Expression::String(now.format("%-I:%M %p").to_string()),
        String::from("date") => Expression::String(now.format("%Y-%m-%d").to_string()),
        String::from("datetime") => Expression::String(now.format("%Y-%m-%d %H:%M:%S").to_string()),
        String::from("rfc3339") => Expression::String(now.to_rfc3339()),
        String::from("rfc2822") => Expression::String(now.to_rfc2822()),
        String::from("week") => Expression::Integer(now.iso_week().week() as i64),
        String::from("ordinal") => Expression::Integer(now.ordinal() as i64),
    }))
}

fn fmt(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("fmt", &args, 1..2)?;

    let format_str = match args[0].eval(env)? {
        Expression::String(s) => s,
        _ => {
            return Err(LmError::CustomError(
                "fmt requires format string as first argument".to_string(),
            ));
        }
    };

    let dt = if args.len() == 2 {
        parse_datetime_arg(&args[1], env)?
    } else {
        Local::now()
    };

    Ok(Expression::String(dt.format(&format_str).to_string()))
}

fn now(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    match args.len() {
        0 => Ok(Expression::Integer(Local::now().timestamp())),
        1 => {
            let format = match args[0].eval(env)? {
                Expression::String(s) => s,
                _ => {
                    return Err(LmError::CustomError(
                        "now requires optional format string".to_string(),
                    ));
                }
            };
            Ok(Expression::String(Local::now().format(&format).to_string()))
        }
        _ => Err(LmError::CustomError(
            "now expects 0 or 1 arguments".to_string(),
        )),
    }
}
fn parse(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("parse", &args, 2)?;

    let datetime_str = match args[0].eval(env)? {
        Expression::String(s) => s,
        _ => {
            return Err(LmError::CustomError(
                "parse requires datetime string as first argument".to_string(),
            ));
        }
    };

    let format_str = match args[1].eval(env)? {
        Expression::String(s) => s,
        _ => {
            return Err(LmError::CustomError(
                "parse requires format string as second argument".to_string(),
            ));
        }
    };

    // Parse the datetime string using NaiveDateTime
    let naive_dt = NaiveDateTime::parse_from_str(&datetime_str, &format_str)
        .map_err(|e| LmError::CustomError(format!("Failed to parse datetime: {}", e)))?;

    // Convert to local DateTime
    let dt: DateTime<Local> = Local
        .from_local_datetime(&naive_dt)
        .single()
        .ok_or_else(|| LmError::CustomError("Failed to convert to local datetime".to_string()))?;

    Ok(Expression::Integer(dt.timestamp()))
}

fn add_duration(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("add_duration", &args, 2..7)?;

    // 解析目标时间（如果提供）
    let base_dt = if args.len() > 2 {
        parse_datetime_arg(&args[2], env)?
    } else {
        Local::now()
    };

    // 解析持续时间
    let duration = if let Expression::String(s) = args[1].eval(env)? {
        parse_duration_string(&s)?
    } else {
        // 兼容旧版：支持单独的时分秒等参数
        let mut duration = ChronoDuration::zero();

        if let Ok(Expression::Integer(secs)) = if args.len() > 2 {
            args[1].eval(env)
        } else {
            args[0].eval(env)
        } {
            duration += ChronoDuration::seconds(secs);
        }

        if args.len() > 3 {
            if let Ok(Expression::Integer(mins)) = args[2].eval(env) {
                duration += ChronoDuration::minutes(mins);
            }
        }

        if args.len() > 4 {
            if let Ok(Expression::Integer(hours)) = args[3].eval(env) {
                duration += ChronoDuration::hours(hours);
            }
        }

        if args.len() > 5 {
            if let Ok(Expression::Integer(days)) = args[4].eval(env) {
                duration += ChronoDuration::days(days);
            }
        }

        duration
            .to_std()
            .map_err(|e| LmError::CustomError(format!("Invalid duration: {}", e)))?
    };

    let result_dt = base_dt + duration;

    // 如果有格式字符串，则格式化输出
    if let Some(format_arg) = args.iter().find(|a| matches!(a, Expression::String(_))) {
        if let Expression::String(format_str) = format_arg.eval(env)? {
            return Ok(Expression::String(
                result_dt.format(&format_str).to_string(),
            ));
        }
    }

    Ok(Expression::Integer(result_dt.timestamp()))
}

fn diff(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("diff", &args, 2..3)?;

    let dt1 = parse_datetime_arg(&args[0], env)?;
    let dt2 = parse_datetime_arg(&args[1], env)?;
    let unit = if args.len() > 2 {
        match args[2].eval(env)? {
            Expression::String(s) => s.to_lowercase(),
            _ => "seconds".to_string(),
        }
    } else {
        "seconds".to_string()
    };

    let duration = if dt1 > dt2 { dt1 - dt2 } else { dt2 - dt1 };

    let value = match unit.as_str() {
        "ms" | "milliseconds" => duration.num_milliseconds(),
        "s" | "seconds" => duration.num_seconds(),
        "m" | "minutes" => duration.num_minutes(),
        "h" | "hours" => duration.num_hours(),
        "d" | "days" => duration.num_days(),
        "w" | "weeks" => duration.num_weeks(),
        _ => duration.num_seconds(),
    };

    Ok(Expression::Integer(value))
}

// fn timer(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
//     super::check_args_len("timer", &args, 1..2)?;

//     let delay = match args[0].eval(env)? {
//         Expression::String(s) => parse_duration_string(&s)?,
//         Expression::Integer(ms) => Duration::from_millis(ms as u64),
//         Expression::Float(secs) => Duration::from_secs_f64(secs),
//         _ => {
//             return Err(LmError::CustomError(
//                 "timer requires duration as first argument".to_string(),
//             ));
//         }
//     };

//     if args.len() == 1 {
//         thread::sleep(delay);
//         return Ok(Expression::None);
//     }

//     // 异步执行回调函数
//     let callback = args[1].clone();
//     let mut env = env.clone();

//     std::thread::spawn(move || {
//         thread::sleep(delay);
//         let _ = callback.eval(&mut env);
//     });

//     Ok(Expression::None)
// }

fn timezone(args: &Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("timezone", &args, 1..3)?;

    let offset_hours = match args[0].eval(env)? {
        Expression::Integer(h) => h,
        Expression::Float(h) => h.round() as i64,
        _ => {
            return Err(LmError::CustomError(
                "timezone requires offset in hours as first argument".to_string(),
            ));
        }
    };

    if !(-12..=14).contains(&offset_hours) {
        return Err(LmError::CustomError(
            "Timezone offset must be between -12 and +14 hours".to_string(),
        ));
    }

    let offset = FixedOffset::east_opt((offset_hours * 3600) as i32)
        .ok_or(LmError::CustomError("Invalid timezone offset".to_string()))?;

    let dt = if args.len() > 1 {
        parse_datetime_arg(&args[1], env)?.with_timezone(&offset)
    } else {
        Local::now().with_timezone(&offset)
    };

    if args.len() > 2 {
        if let Expression::String(format) = args[2].eval(env)? {
            return Ok(Expression::String(dt.format(&format).to_string()));
        }
    }

    Ok(Expression::String(dt.to_rfc3339()))
}
