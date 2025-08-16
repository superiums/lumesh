use crate::{Environment, Expression, LmError};
use chrono::{
    DateTime, Datelike, Duration as ChronoDuration, FixedOffset, Local, NaiveDate, NaiveDateTime,
    NaiveTime, TimeZone, Timelike, Utc,
};
use common_macros::hash_map;
use std::{collections::BTreeMap, thread, time::Duration};

pub fn get() -> Expression {
    (hash_map! {
        // 基本时间获取
        String::from("sleep") => Expression::builtin("sleep", sleep, "sleep for a given number of milliseconds [ms] or duration string (e.g. '1s', '2m')", "<duration>"),
        String::from("display") => Expression::builtin("display", display, "get preformatted datetime as map with time/date/datetime/etc.", "[datetime]"),

        // 时间分量获取（参数格式统一为 [datetime]）
        String::from("year") => Expression::builtin("year", |args, env| get_time_component(args, env, |dt| dt.year() as i64), "get year (current or from specified datetime)", "[datetime]"),
        String::from("month") => Expression::builtin("month", |args, env| get_time_component(args, env, |dt| dt.month() as i64), "get month (1-12)", "[datetime]"),
        String::from("weekday") => Expression::builtin("weekday", |args, env| get_time_component(args, env, |dt| dt.weekday().num_days_from_monday() as i64 + 1), "get weekday (1-7, Monday=1)", "[datetime]"),
        String::from("day") => Expression::builtin("day", |args, env| get_time_component(args, env, |dt| dt.day() as i64), "get day of month (1-31)", "[datetime]"),
        String::from("hour") => Expression::builtin("hour", |args, env| get_time_component(args, env, |dt| dt.hour() as i64), "get hour (0-23)", "[datetime]"),
        String::from("minute") => Expression::builtin("minute", |args, env| get_time_component(args, env, |dt| dt.minute() as i64), "get minute (0-59)", "[datetime]"),
        String::from("second") => Expression::builtin("second", |args, env| get_time_component(args, env, |dt| dt.second() as i64), "get second (0-59)", "[datetime]"),
        String::from("seconds") => Expression::builtin("seconds", |args, env| get_time_component(args, env, |dt| dt.time().num_seconds_from_midnight() as i64), "get seconds since midnight", "[datetime]"),

        // 时间戳
        String::from("stamp") => Expression::builtin("stamp", |args, env| {
            let dt = if args.is_empty() { Utc::now() } else { parse_datetime_arg(&args[0], env)?.and_utc() };
            Ok(Expression::Integer(dt.timestamp()))
        }, "get Unix timestamp in seconds", "[datetime]"),
        String::from("stamp_ms") => Expression::builtin("stamp_ms", |args, env| {
            let dt = if args.is_empty() { Utc::now() } else { parse_datetime_arg(&args[0], env)?.and_utc() };
            Ok(Expression::Integer(dt.timestamp_millis()))
        }, "get Unix timestamp in milliseconds", "[datetime]"),

        // 格式化
        String::from("fmt") => Expression::builtin("fmt", fmt, "format datetime (current or specified) using chrono format string", "<format_string> [datetime]"),

        // 核心操作
        String::from("now") => Expression::builtin("now", now, "get current datetime as DateTime object or formatted string", "[format_string]"),
        String::from("parse") => Expression::builtin("parse", parse_time, "parse datetime string according to format", "[format_string] <datetime_string>"),
        String::from("add") => Expression::builtin("add", add, "add duration to datetime", "<duration> <datetime>"),
        String::from("diff") => Expression::builtin("diff", diff, "calculate difference between two datetimes", "<unit> <datetime1> <datetime2>"),
        String::from("timezone") => Expression::builtin("timezone", timezone, "convert datetime to different timezone", "<offset_hours> <datetime>"),
        String::from("is_leap_year") => Expression::builtin("is_leap_year", |args, env| {
            let year = match args.first().map(|a| a.eval(env)) {
                Some(Ok(Expression::Integer(y))) => y,
                Some(Ok(_)) => return Err(LmError::CustomError("Year must be an integer".to_string())),
                Some(Err(e)) => return Err(e.into()),
                None => Local::now().year() as i64,
            };
            Ok(Expression::Boolean(NaiveDate::from_ymd_opt(year as i32, 1, 1)
                .map(|d| d.leap_year())
                .unwrap_or(false)))
        }, "check if a year is a leap year", "[year]"),
        String::from("from_map") => Expression::builtin("from_map", from_map, "create DateTime from components", "<map>"),
        String::from("to_string") => Expression::builtin("to_string", to_string, "convert DateTime to string", "[format_string] <datetime>")

    })
    .into()
}

// 辅助函数实现

/// 获取时间组件（支持从指定时间或当前时间）
fn get_time_component<F>(
    args: &[Expression],
    env: &mut Environment,
    extractor: F,
) -> Result<Expression, LmError>
where
    F: Fn(NaiveDateTime) -> i64,
{
    match args.len() {
        0 => Ok(Expression::Integer(extractor(Local::now().naive_local()))),
        1 => {
            let dt = parse_datetime_arg(&args[0], env)?;
            Ok(Expression::Integer(extractor(dt)))
        }
        _ => Err(LmError::CustomError(
            "Expected 0 or 1 arguments".to_string(),
        )),
    }
}

/// 解析日期时间参数（支持多种格式）
fn parse_datetime_arg(arg: &Expression, env: &mut Environment) -> Result<NaiveDateTime, LmError> {
    match arg.eval(env)? {
        Expression::DateTime(dt) => Ok(dt),
        Expression::String(s) => {
            // 尝试解析常见的shell日期格式
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y/%m/%d %H:%M:%S") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%d/%m/%Y %H:%M:%S") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%m/%d/%Y %H:%M:%S") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %I:%M %p") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y/%m/%d %I:%M %p") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%d/%m/%Y %I:%M %p") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%m/%d/%Y %I:%M %p") {
                return Ok(dt);
            }
            if let Ok(dt) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                return Ok(dt.and_hms_opt(0, 0, 0).unwrap());
            }
            if let Ok(time) = NaiveTime::parse_from_str(&s, "%H:%M:%S") {
                return Ok(NaiveDateTime::new(Local::now().date_naive(), time));
            }
            if let Ok(time) = NaiveTime::parse_from_str(&s, "%H:%M") {
                return Ok(NaiveDateTime::new(Local::now().date_naive(), time));
            }
            if let Ok(time) = NaiveTime::parse_from_str(&s, "%I:%M %p") {
                return Ok(NaiveDateTime::new(Local::now().date_naive(), time));
            }
            // 尝试解析常见格式
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return Ok(dt.naive_local());
            }
            if let Ok(dt) = DateTime::parse_from_rfc2822(&s) {
                return Ok(dt.naive_local());
            }
            if let Ok(ts) = s.parse::<i64>() {
                return Ok(Utc.timestamp_opt(ts, 0).unwrap().naive_utc());
            }
            Err(LmError::CustomError(format!(
                "Unrecognized datetime format: {s}"
            )))
        }
        Expression::Integer(ts) => Ok(Utc.timestamp_opt(ts, 0).unwrap().naive_utc()),
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
                .ok_or(LmError::CustomError("Invalid date components".to_string()))
        }
        _ => Err(LmError::CustomError(
            "Expected DateTime, string, timestamp or map for datetime".to_string(),
        )),
    }
}

fn get_map_value(map: &BTreeMap<String, Expression>, key: &str) -> Result<Option<i64>, LmError> {
    match map.get(key) {
        Some(Expression::Integer(n)) => Ok(Some(*n)),
        Some(Expression::String(s)) => s
            .parse()
            .map(Some)
            .map_err(|_| LmError::CustomError(format!("Invalid integer value for {key}"))),
        None => Ok(None),
        _ => Err(LmError::CustomError(format!("Expected integer for {key}"))),
    }
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
                    return Err(LmError::CustomError(format!("Unknown duration unit: {c}")));
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

// 主要功能函数实现

fn sleep(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("sleep", args, 1)?;

    let duration = match args[0].eval(env)? {
        Expression::Float(n) if n > 0.0 => Duration::from_millis(n as u64),
        Expression::Integer(n) if n > 0 => Duration::from_millis(n as u64),
        Expression::String(s) => parse_duration_string(&s)?,
        otherwise => {
            return Err(LmError::CustomError(format!(
                "expected positive number or duration string, got {otherwise}"
            )));
        }
    };

    thread::sleep(duration);
    Ok(Expression::None)
}

fn display(_: &[Expression], _: &mut Environment) -> Result<Expression, LmError> {
    let now = Local::now();
    let naive = now.naive_local();
    Ok(Expression::from(hash_map! {
        String::from("time") => Expression::String(naive.time().format("%H:%M:%S").to_string()),
        String::from("timepm") => Expression::String(naive.format("%-I:%M %p").to_string()),
        String::from("date") => Expression::String(naive.date().format("%Y-%m-%d").to_string()),
        String::from("datetime") => Expression::String(naive.format("%Y-%m-%d %H:%M:%S").to_string()),
        String::from("rfc3339") => Expression::String(now.to_rfc3339()),
        String::from("rfc2822") => Expression::String(now.to_rfc2822()),
        String::from("week") => Expression::Integer(naive.iso_week().week() as i64),
        String::from("ordinal") => Expression::Integer(naive.ordinal() as i64),
        String::from("datetime_obj") => Expression::DateTime(naive),
    }))
}

fn fmt(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("fmt", args, 1..=2)?;

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
        Local::now().naive_local()
    };

    Ok(Expression::String(dt.format(&format_str).to_string()))
}

fn now(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    match args.len() {
        0 => Ok(Expression::DateTime(Local::now().naive_local())),
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

pub fn parse_time(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("parse", args, 1..=2)?;

    let datetime_str = match args.last().unwrap().eval(env)? {
        Expression::String(s) => s,
        _ => {
            return Err(LmError::CustomError(
                "parse requires datetime string as first argument".to_string(),
            ));
        }
    };

    let format_str = if args.len() > 1 {
        match args[0].eval(env)? {
            Expression::String(s) => s,
            _ => {
                return Err(LmError::CustomError(
                    "parse requires format string as second argument".to_string(),
                ));
            }
        }
    } else {
        // "%Y-%m-%d %H:%M:%S".to_owned()
        return Ok(Expression::DateTime(parse_datetime_arg(
            &Expression::String(datetime_str),
            env,
        )?));
    };

    // 尝试解析为 NaiveDateTime
    if let Ok(dt) = NaiveDateTime::parse_from_str(&datetime_str, &format_str) {
        return Ok(Expression::DateTime(dt));
    }

    // 尝试解析为 NaiveDate
    if let Ok(date) = NaiveDate::parse_from_str(&datetime_str, &format_str) {
        return Ok(Expression::DateTime(date.and_hms_opt(0, 0, 0).unwrap()));
    }

    // 尝试解析为 NaiveTime
    if let Ok(time) = NaiveTime::parse_from_str(&datetime_str, &format_str) {
        let today = Local::now().date_naive();
        return Ok(Expression::DateTime(today.and_time(time)));
    }

    Err(LmError::CustomError(format!(
        "Failed to parse datetime '{datetime_str}' with format '{format_str}'"
    )))
}

fn add(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("add", args, 1..=7)?;

    // 解析目标时间（如果提供）
    let base_dt = if args.len() > 1 {
        parse_datetime_arg(&args[1], env)?
    } else {
        Local::now().naive_local()
    };

    // 解析持续时间
    let duration = if let Expression::String(s) = args[0].eval(env)? {
        let std_duration = parse_duration_string(&s)?;
        ChronoDuration::from_std(std_duration)
            .map_err(|e| LmError::CustomError(format!("Invalid duration: {e}")))?
    } else {
        // 兼容旧版：支持单独的时分秒等参数
        let mut duration = ChronoDuration::zero();

        if args.len() > 1 {
            if let Ok(Expression::Integer(secs)) = args[0].eval(env) {
                duration += ChronoDuration::seconds(secs);
            }
        }

        if args.len() > 2 {
            if let Ok(Expression::Integer(mins)) = args[1].eval(env) {
                duration += ChronoDuration::minutes(mins);
            }
        }

        if args.len() > 3 {
            if let Ok(Expression::Integer(hours)) = args[2].eval(env) {
                duration += ChronoDuration::hours(hours);
            }
        }

        if args.len() > 4 {
            if let Ok(Expression::Integer(days)) = args[3].eval(env) {
                duration += ChronoDuration::days(days);
            }
        }

        duration
    };

    let result_dt = base_dt + duration;

    // 如果有格式字符串，则格式化输出
    // if let Some(format_arg) = args.iter().find(|a| matches!(a, Expression::String(_))) {
    //     if let Expression::String(format_str) = format_arg.eval(env)? {
    //         return Ok(Expression::String(
    //             result_dt.format(&format_str).to_string(),
    //         ));
    //     }
    // }

    Ok(Expression::DateTime(result_dt))
}

fn diff(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("diff", args, 2..=3)?;
    let unit = match args[0].eval(env)? {
        Expression::String(s) => s.to_lowercase(),
        _ => "seconds".to_string(),
    };

    let dt1 = parse_datetime_arg(&args[1], env)?;

    let dt2 = if args.len() > 2 {
        parse_datetime_arg(&args[2], env)?
    } else {
        Local::now().naive_local()
    };

    let duration = dt2 - dt1;

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

fn timezone(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("timezone", args, 1..=3)?;

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
        let naive = parse_datetime_arg(&args[1], env)?;
        offset.from_utc_datetime(&naive).naive_local()
    } else {
        Local::now().with_timezone(&offset).naive_local()
    };

    if args.len() > 2 {
        if let Expression::String(format) = args[2].eval(env)? {
            return Ok(Expression::String(dt.format(&format).to_string()));
        }
    }

    Ok(Expression::DateTime(dt))
}

// 新增功能函数

fn from_map(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("from_map", args, 1)?;

    // 处理 Map 类型参数

    if let Expression::Map(m) = args[0].eval(env)? {
        let map = m.as_ref();
        let year = get_map_value(map, "year")?
            .ok_or(LmError::CustomError("Missing year".to_string()))? as i32;
        let month = get_map_value(map, "month")?
            .ok_or(LmError::CustomError("Missing month".to_string()))? as u32;
        let day = get_map_value(map, "day")?
            .ok_or(LmError::CustomError("Missing day".to_string()))? as u32;

        let hour = get_map_value(map, "hour")?.unwrap_or(0) as u32;
        let minute = get_map_value(map, "minute")?.unwrap_or(0) as u32;
        let second = get_map_value(map, "second")?.unwrap_or(0) as u32;

        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or(LmError::CustomError("Invalid date components".to_string()))?;

        let datetime = date
            .and_hms_opt(hour, minute, second)
            .ok_or(LmError::CustomError("Invalid time components".to_string()))?;

        Ok(Expression::DateTime(datetime))
    } else {
        Err(LmError::CustomError(
            "a map is required for time.from_part".to_string(),
        ))
    }
}

fn to_string(args: &[Expression], env: &mut Environment) -> Result<Expression, LmError> {
    super::check_args_len("to_string", args, 1..=2)?;

    let dt = parse_datetime_arg(&args[0], env)?;

    if args.len() == 1 {
        // 默认使用RFC3339格式
        Ok(Expression::String(
            dt.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        ))
    } else {
        match args[1].eval(env)? {
            Expression::String(format) => Ok(Expression::String(dt.format(&format).to_string())),
            _ => Err(LmError::CustomError("Expected format string".to_string())),
        }
    }
}
