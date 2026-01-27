use crate::{Environment, Expression};
use chrono::{
    DateTime, Datelike, Duration as ChronoDuration, FixedOffset, Local, NaiveDate, NaiveDateTime,
    NaiveTime, TimeZone, Timelike, Utc,
};
use common_macros::hash_map;
use std::{collections::BTreeMap, thread, time::Duration};

use crate::libs::helper::{check_args_len, check_exact_args_len};
use crate::libs::lazy_module::LazyModule;
use crate::{RuntimeError, libs::BuiltinInfo, reg_info, reg_lazy};

pub fn regist_lazy() -> LazyModule {
    reg_lazy!({
        // 基本时间获取
        sleep, display,
        // 时间分量获取（参数格式统一为 [datetime]）
        year, month, weekday, day, hour, minute, second, seconds,
        // 时间戳
        stamp, stamp_ms,
        // 格式化
        fmt,
        // 核心操作
        now, parse, add, diff, timezone, is_leap, from_map, to_string,
    })
}
pub fn regist_info() -> BTreeMap<&'static str, BuiltinInfo> {
    reg_info!({
        // 基本时间获取
        sleep => "sleep for a given number of milliseconds [ms] or duration string (e.g. '1s', '2m')", "<duration>"
        display => "get preformatted datetime as map with time/date/datetime/etc.", "[datetime]"

                // 时间分量获取（参数格式统一为 [datetime]）
        year => "get year (current or from specified datetime)", "[datetime]"
        month => "get month (1-12)", "[datetime]"
        weekday => "get weekday (1-7, Monday=1)", "[datetime]"
        day => "get day of month (1-31)", "[datetime]"
        hour => "get hour (0-23)", "[datetime]"
        minute => "get minute (0-59)", "[datetime]"
        second => "get second (0-59)", "[datetime]"
        seconds => "get seconds since midnight", "[datetime]"

                // 时间戳
        stamp => "get Unix timestamp in seconds", "[datetime]"
        stamp_ms => "get Unix timestamp in milliseconds", "[datetime]"

                // 格式化
        fmt => "format datetime (current or specified) using chrono format string", "<format_string> [datetime]"

                // 核心操作
        now => "get current datetime as DateTime object or formatted string", "[format_string]"
        parse => "parse datetime string according to format", "[format_string] <datetime_string>"
        add => "add duration to datetime", "<duration> <datetime>"
        diff => "calculate difference between two datetimes", "<unit> <datetime1> <datetime2>"
        timezone => "convert datetime to different timezone", "<offset_hours> <datetime>"
        is_leap_year => "check if a year is a leap year", "[year]"
        from_map => "create DateTime from components", "<map>"
        to_string => "convert DateTime to string", "[format_string] <datetime>"
    })
}
// Helper Functions
fn parse_datetime_arg(
    arg: &Expression,
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<NaiveDateTime, RuntimeError> {
    match arg {
        Expression::DateTime(dt) => Ok(dt.clone()),
        Expression::String(s) => {
            // Try parsing common shell date formats
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
            // Try parsing common formats
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return Ok(dt.naive_local());
            }
            if let Ok(dt) = DateTime::parse_from_rfc2822(&s) {
                return Ok(dt.naive_local());
            }
            if let Ok(ts) = s.parse::<i64>() {
                return Ok(Utc.timestamp_opt(ts, 0).unwrap().naive_utc());
            }
            Err(RuntimeError::common(
                format!("Unrecognized datetime format: {s}").into(),
                ctx.clone(),
                0,
            ))
        }
        Expression::Integer(ts) => Ok(Utc.timestamp_opt(*ts, 0).unwrap().naive_utc()),
        Expression::Map(m) => {
            let map = m.as_ref();
            let year = get_map_value(map, "year", ctx)?.unwrap_or(Local::now().year() as i64);
            let month = get_map_value(map, "month", ctx)?.unwrap_or(1) as u32;
            let day = get_map_value(map, "day", ctx)?.unwrap_or(1) as u32;
            let hour = get_map_value(map, "hour", ctx)?.unwrap_or(0) as u32;
            let minute = get_map_value(map, "minute", ctx)?.unwrap_or(0) as u32;
            let second = get_map_value(map, "second", ctx)?.unwrap_or(0) as u32;

            NaiveDate::from_ymd_opt(year as i32, month, day)
                .and_then(|d| d.and_hms_opt(hour, minute, second))
                .ok_or(RuntimeError::common(
                    "Invalid date components".into(),
                    ctx.clone(),
                    0,
                ))
        }
        _ => Err(RuntimeError::common(
            "Expected DateTime, string, timestamp or map for datetime".into(),
            ctx.clone(),
            0,
        )),
    }
}

fn get_map_value(
    map: &BTreeMap<String, Expression>,
    key: &str,
    ctx: &Expression,
) -> Result<Option<i64>, RuntimeError> {
    match map.get(key) {
        Some(Expression::Integer(n)) => Ok(Some(*n)),
        Some(Expression::String(s)) => s.parse().map(Some).map_err(|_| {
            RuntimeError::common(
                format!("Invalid integer value for {key}").into(),
                ctx.clone(),
                0,
            )
        }),
        None => Ok(None),
        _ => Err(RuntimeError::common(
            format!("Expected integer for {key}").into(),
            ctx.clone(),
            0,
        )),
    }
}

fn parse_duration_string(s: &str, ctx: &Expression) -> Result<Duration, RuntimeError> {
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
                    return Err(RuntimeError::common(
                        format!("Unknown duration unit: {c}").into(),
                        ctx.clone(),
                        0,
                    ));
                }
            };
            total_ms += num * multiplier;
            num = 0;
        }
    }

    // Handle case without unit, default to milliseconds
    if num > 0 {
        total_ms += num;
    }

    Ok(Duration::from_millis(total_ms))
}
// Basic Time Functions
fn sleep(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("sleep", args, 1, ctx)?;

    let duration = match &args[0] {
        Expression::Float(n) if *n > 0.0 => Duration::from_millis(*n as u64),
        Expression::Integer(n) if *n > 0 => Duration::from_millis(*n as u64),
        Expression::String(s) => parse_duration_string(&s, ctx)?,
        otherwise => {
            return Err(RuntimeError::common(
                format!("expected positive number or duration string, got {otherwise}").into(),
                ctx.clone(),
                0,
            ));
        }
    };

    thread::sleep(duration);
    Ok(Expression::None)
}

fn display(
    _args: &[Expression],
    _env: &mut Environment,
    _ctx: &Expression,
) -> Result<Expression, RuntimeError> {
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
// Time Component Functions
fn get_time_component<F>(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
    extractor: F,
) -> Result<Expression, RuntimeError>
where
    F: Fn(NaiveDateTime) -> i64,
{
    match args.len() {
        0 => Ok(Expression::Integer(extractor(Local::now().naive_local()))),
        1 => {
            let dt = parse_datetime_arg(&args[0], env, ctx)?;
            Ok(Expression::Integer(extractor(dt)))
        }
        _ => Err(RuntimeError::common(
            "Expected 0 or 1 arguments".into(),
            ctx.clone(),
            0,
        )),
    }
}

fn year(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    get_time_component(args, env, ctx, |dt| dt.year() as i64)
}

fn month(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    get_time_component(args, env, ctx, |dt| dt.month() as i64)
}

fn weekday(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    get_time_component(args, env, ctx, |dt| {
        dt.weekday().num_days_from_monday() as i64 + 1
    })
}

fn day(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    get_time_component(args, env, ctx, |dt| dt.day() as i64)
}

fn hour(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    get_time_component(args, env, ctx, |dt| dt.hour() as i64)
}

fn minute(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    get_time_component(args, env, ctx, |dt| dt.minute() as i64)
}

fn second(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    get_time_component(args, env, ctx, |dt| dt.second() as i64)
}

fn seconds(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    get_time_component(args, env, ctx, |dt| {
        dt.time().num_seconds_from_midnight() as i64
    })
}
fn is_leap(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let year = match args.first() {
        Some(Expression::Integer(y)) => *y,
        Some(_) => {
            return Err(RuntimeError::common(
                "Year must be an integer".into(),
                ctx.clone(),
                0,
            ));
        }
        None => Local::now().year() as i64,
    };
    Ok(Expression::Boolean(
        NaiveDate::from_ymd_opt(year as i32, 1, 1)
            .map(|d| d.leap_year())
            .unwrap_or(false),
    ))
}

// Timestamp Functions
fn stamp(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let dt = if args.is_empty() {
        Utc::now()
    } else {
        parse_datetime_arg(&args[0], env, ctx)?.and_utc()
    };
    Ok(Expression::Integer(dt.timestamp()))
}

fn stamp_ms(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    let dt = if args.is_empty() {
        Utc::now()
    } else {
        parse_datetime_arg(&args[0], env, ctx)?.and_utc()
    };
    Ok(Expression::Integer(dt.timestamp_millis()))
}
// Formatting Functions
fn fmt(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("fmt", args, 1..=2, ctx)?;

    let format_str = match &args[0] {
        Expression::String(s) => s,
        _ => {
            return Err(RuntimeError::common(
                "fmt requires format string as first argument".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let dt = if args.len() == 2 {
        parse_datetime_arg(&args[1], env, ctx)?
    } else {
        Local::now().naive_local()
    };

    Ok(Expression::String(dt.format(&format_str).to_string()))
}

fn now(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    match args.len() {
        0 => Ok(Expression::DateTime(Local::now().naive_local())),
        1 => {
            let format = match &args[0] {
                Expression::String(s) => s,
                _ => {
                    return Err(RuntimeError::common(
                        "now requires optional format string".into(),
                        ctx.clone(),
                        0,
                    ));
                }
            };
            Ok(Expression::String(Local::now().format(&format).to_string()))
        }
        _ => Err(RuntimeError::common(
            "now expects 0 or 1 arguments".into(),
            ctx.clone(),
            0,
        )),
    }
}

fn to_string(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("to_string", args, 1..=2, ctx)?;

    let dt = parse_datetime_arg(&args[0], env, ctx)?;

    if args.len() == 1 {
        // Default to RFC3339 format
        Ok(Expression::String(
            dt.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        ))
    } else {
        match &args[1] {
            Expression::String(format) => Ok(Expression::String(dt.format(&format).to_string())),
            _ => Err(RuntimeError::common(
                "Expected format string".into(),
                ctx.clone(),
                0,
            )),
        }
    }
}
// Parsing and Creation Functions
pub fn parse(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("parse", args, 1..=2, ctx)?;

    let datetime_str = match &args[0] {
        Expression::String(s) => s,
        _ => {
            return Err(RuntimeError::common(
                "parse requires datetime string as first argument".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    let format_str = if args.len() > 1 {
        match &args[1] {
            Expression::String(s) => s,
            _ => {
                return Err(RuntimeError::common(
                    "parse requires format string as second argument".into(),
                    ctx.clone(),
                    0,
                ));
            }
        }
    } else {
        // Try to parse without format
        return Ok(Expression::DateTime(parse_datetime_arg(
            &Expression::String(datetime_str.clone()),
            env,
            ctx,
        )?));
    };

    // Try parsing as NaiveDateTime
    if let Ok(dt) = NaiveDateTime::parse_from_str(&datetime_str, &format_str) {
        return Ok(Expression::DateTime(dt));
    }

    // Try parsing as NaiveDate
    if let Ok(date) = NaiveDate::parse_from_str(&datetime_str, &format_str) {
        return Ok(Expression::DateTime(date.and_hms_opt(0, 0, 0).unwrap()));
    }

    // Try parsing as NaiveTime
    if let Ok(time) = NaiveTime::parse_from_str(&datetime_str, &format_str) {
        let today = Local::now().date_naive();
        return Ok(Expression::DateTime(today.and_time(time)));
    }

    Err(RuntimeError::common(
        format!("Failed to parse datetime '{datetime_str}' with format '{format_str}'").into(),
        ctx.clone(),
        0,
    ))
}

fn from_map(
    args: &[Expression],
    _env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_exact_args_len("from_map", args, 1, ctx)?;

    if let Expression::Map(m) = &args[0] {
        let map = m.as_ref();
        let year = get_map_value(map, "year", ctx)?.ok_or(RuntimeError::common(
            "Missing year".into(),
            ctx.clone(),
            0,
        ))? as i32;
        let month = get_map_value(map, "month", ctx)?.ok_or(RuntimeError::common(
            "Missing month".into(),
            ctx.clone(),
            0,
        ))? as u32;
        let day = get_map_value(map, "day", ctx)?.ok_or(RuntimeError::common(
            "Missing day".into(),
            ctx.clone(),
            0,
        ))? as u32;

        let hour = get_map_value(map, "hour", ctx)?.unwrap_or(0) as u32;
        let minute = get_map_value(map, "minute", ctx)?.unwrap_or(0) as u32;
        let second = get_map_value(map, "second", ctx)?.unwrap_or(0) as u32;

        let date = NaiveDate::from_ymd_opt(year, month, day).ok_or(RuntimeError::common(
            "Invalid date components".into(),
            ctx.clone(),
            0,
        ))?;

        let datetime = date
            .and_hms_opt(hour, minute, second)
            .ok_or(RuntimeError::common(
                "Invalid time components".into(),
                ctx.clone(),
                0,
            ))?;

        Ok(Expression::DateTime(datetime))
    } else {
        Err(RuntimeError::common(
            "a map is required for time.from_map".into(),
            ctx.clone(),
            0,
        ))
    }
}
// Time Arithmetic Functions
fn add(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("add", args, 1..=7, ctx)?;
    let (base_dt, rest) = match args.len() > 1 {
        false => (Local::now().naive_local(), &args[1..]),
        true => (parse_datetime_arg(&args[1], env, ctx)?, &args[1..]),
    };

    let duration = match rest.len() {
        0 => ChronoDuration::zero(),
        1 => match &rest[0] {
            Expression::String(dur) => {
                let std_duration = parse_duration_string(&dur, ctx)?;
                ChronoDuration::from_std(std_duration).map_err(|e| {
                    RuntimeError::common(format!("Invalid duration: {e}").into(), ctx.clone(), 0)
                })?
            }
            Expression::Integer(secs) => ChronoDuration::seconds(*secs),
            e => {
                return Err(RuntimeError::common(
                    format!("Invalid duration: {e}").into(),
                    ctx.clone(),
                    0,
                ));
            }
        },
        2.. => {
            let mut duration = ChronoDuration::zero();

            if let Expression::Integer(secs) = rest[0] {
                duration += ChronoDuration::seconds(secs);
            }

            if let Expression::Integer(mins) = rest[1] {
                duration += ChronoDuration::minutes(mins);
            }

            if rest.len() > 2 {
                if let Expression::Integer(hours) = rest[2] {
                    duration += ChronoDuration::hours(hours);
                }
            }

            if rest.len() > 3 {
                if let Expression::Integer(days) = rest[3] {
                    duration += ChronoDuration::days(days);
                }
            }

            duration
        }
    };

    let result_dt = base_dt + duration;
    Ok(Expression::DateTime(result_dt))
}

fn diff(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("diff", args, 2..=3, ctx)?;

    let unit = match &args[0] {
        Expression::String(s) => s.to_lowercase(),
        _ => "seconds".to_string(),
    };

    let dt1 = parse_datetime_arg(&args[1], env, ctx)?;

    let dt2 = if args.len() > 2 {
        parse_datetime_arg(&args[2], env, ctx)?
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

// Timezone Functions
fn timezone(
    args: &[Expression],
    env: &mut Environment,
    ctx: &Expression,
) -> Result<Expression, RuntimeError> {
    check_args_len("timezone", args, 1..=3, ctx)?;

    let offset_hours = match &args[0] {
        Expression::Integer(h) => *h,
        Expression::Float(h) => (*h).round() as i64,
        _ => {
            return Err(RuntimeError::common(
                "timezone requires offset in hours as first argument".into(),
                ctx.clone(),
                0,
            ));
        }
    };

    if !(-12..=14).contains(&offset_hours) {
        return Err(RuntimeError::common(
            "Timezone offset must be between -12 and +14 hours".into(),
            ctx.clone(),
            0,
        ));
    }

    let offset = FixedOffset::east_opt((offset_hours * 3600) as i32).ok_or(
        RuntimeError::common("Invalid timezone offset".into(), ctx.clone(), 0),
    )?;

    let dt = if args.len() > 1 {
        let naive = parse_datetime_arg(&args[1], env, ctx)?;
        offset.from_utc_datetime(&naive).naive_local()
    } else {
        Local::now().with_timezone(&offset).naive_local()
    };

    if args.len() > 2 {
        if let Expression::String(format) = &args[2] {
            return Ok(Expression::String(dt.format(&format).to_string()));
        }
    }

    Ok(Expression::DateTime(dt))
}
