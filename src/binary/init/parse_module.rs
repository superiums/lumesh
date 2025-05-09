use crate::{Environment, Expression, LmError, SyntaxError, SyntaxErrorKind, parse_script};
use common_macros::hash_map;
use json::JsonValue;
use std::collections::HashMap;

pub fn get() -> Expression {
    (hash_map! {
        String::from("toml") => Expression::builtin("toml", parse_toml, "parse a TOML value into a lumesh expression"),
        String::from("json") => Expression::builtin("json", parse_json, "parse a JSON value into a lumesh expression"),
        String::from("expr") => Expression::builtin("expr", parse_expr, "parse a lumesh script"),
    })
    .into()
}

fn parse_expr(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("expr", &args, 1)?;
    let script = args[0].eval(env)?.to_string();
    if script.is_empty() {
        return Ok(Expression::None);
    }
    match parse_script(&script) {
        Ok(val) => Ok(val),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(SyntaxError {
            source: script.into(),
            kind: e,
        })?,
        Err(nom::Err::Incomplete(_)) => Err(SyntaxError {
            source: script.into(),
            kind: SyntaxErrorKind::InternalError,
        })?,
    }
}

fn parse_json(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("json", &args, 1)?;
    let text = args[0].eval(env)?.to_string();
    if text.is_empty() {
        return Ok(Expression::None);
    }

    if let Ok(val) = json::parse(&text) {
        Ok(json_to_expr(val))
    } else {
        Err(LmError::CustomError(format!(
            "could not parse `{}` as JSON",
            text
        )))
    }
}

fn json_to_expr(val: JsonValue) -> Expression {
    match val {
        JsonValue::Null => Expression::None,
        JsonValue::Boolean(b) => Expression::Boolean(b),
        JsonValue::Number(n) => Expression::Float(n.into()),
        JsonValue::Short(s) => Expression::String(s.to_string()),
        JsonValue::String(s) => Expression::String(s),
        JsonValue::Array(a) => {
            let mut v = Vec::new();
            for e in a {
                v.push(json_to_expr(e));
            }
            Expression::from(v)
        }
        JsonValue::Object(o) => {
            let mut m = HashMap::new();
            for (k, v) in o.iter() {
                m.insert(k.to_string(), json_to_expr(v.clone()));
            }
            Expression::from(m)
        }
    }
}

fn parse_toml(args: Vec<Expression>, env: &mut Environment) -> Result<Expression, LmError> {
    super::check_exact_args_len("toml", &args, 1)?;
    let text = args[0].eval(env)?.to_string();
    if let Ok(val) = text.parse::<toml::Value>() {
        Ok(toml_to_expr(val))
    } else {
        Err(LmError::CustomError(format!(
            "could not parse `{}` as TOML",
            text
        )))
    }
}

fn toml_to_expr(val: toml::Value) -> Expression {
    match val {
        toml::Value::Boolean(b) => Expression::Boolean(b),
        toml::Value::Float(n) => Expression::Float(n),
        toml::Value::Integer(n) => Expression::Integer(n),
        toml::Value::Datetime(s) => Expression::String(s.to_string()),
        toml::Value::String(s) => Expression::String(s),
        toml::Value::Array(a) => {
            let mut v = Vec::new();
            for e in a {
                v.push(toml_to_expr(e));
            }
            Expression::from(v)
        }
        toml::Value::Table(o) => {
            let mut m = HashMap::new();
            for (k, v) in o.iter() {
                m.insert(k.to_string(), toml_to_expr(v.clone()));
            }
            Expression::from(m)
        }
    }
}
