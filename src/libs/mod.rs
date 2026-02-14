mod bin;
mod helper;
mod lazy_module;
mod pprint;
use crate::{Environment, Expression, RuntimeError, eval::State, libs::lazy_module::LazyModule};
pub use bin::time_lib::parse as time_parse;
pub use bin::top::regist_info;
pub use pprint::pretty_printer;
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};

pub struct BuiltinInfo {
    pub descr: &'static str,
    pub hint: &'static str,
}
pub type BuiltinFunc =
    fn(Vec<Expression>, &mut Environment, contex: &Expression) -> Result<Expression, RuntimeError>;

pub type SelfExpandFunc = fn(
    &[Expression],
    &mut Environment,
    &mut State,
    contex: &Expression,
) -> Result<Expression, RuntimeError>;

// 对不同模块采用不同策略
thread_local! {
    // 帮助信息，初次使用时加载
    pub static LIBS_INFO: LazyLock<BTreeMap<&'static str, BTreeMap<&'static str,BuiltinInfo>>> =LazyLock::new(regist_all_info);

    // 热模块/小型模块：完全预加载
    // static MATH_LIB: RefCell<HashMap<String, Expression>> = RefCell::new({
    //     math_module::get_all_functions() // 加载所有函数
    // });
    static SE_LIB: RefCell<HashMap<&'static str, SelfExpandFunc>> = RefCell::new({
        bin::se_lib::regist_se()
    });

    static TOP_LIB: RefCell<HashMap<&'static str, BuiltinFunc>> = RefCell::new({
        bin::top::regist_all()
    });
    static BOOL_LIB: RefCell<HashMap<&'static str, BuiltinFunc>> = RefCell::new({
        bin::boolean_lib::regist_all()
    });


    // 中型模块：模块级懒加载
    // static FS_LIB: RefCell<Option<Expression>> = RefCell::new(None);


    // 大型模块：函数级懒加载
    static STRING_LIB: LazyModule = bin::string_lib::regist_lazy();
    static LIST_LIB: LazyModule = bin::list_lib::regist_lazy();
    static BSET_LIB: LazyModule = bin::bset_lib::regist_lazy();
    static MAP_LIB: LazyModule = bin::map_lib::regist_lazy();
    static HMAP_LIB: LazyModule = bin::hmap_lib::regist_lazy();
    static TIME_LIB: LazyModule = bin::time_lib::regist_lazy();
    static REGEX_LIB: LazyModule = bin::reg_lib::regist_lazy();
    static MATH_LIB: LazyModule = bin::math_lib::regist_lazy();
    static RAND_LIB: LazyModule = bin::rand_lib::regist_lazy();
    static LOG_LIB: LazyModule = bin::log_lib::regist_lazy();
    static FS_LIB: LazyModule = bin::fs_lib::regist_lazy();
    static UI_LIB: LazyModule = bin::ui_lib::regist_lazy();
    static INTO_LIB: LazyModule = bin::into_lib::regist_lazy();
    static SYS_LIB: LazyModule = bin::sys_lib::regist_lazy();
    static FILESIZE_LIB: LazyModule = bin::filesize_lib::regist_lazy();
    static FROM_LIB: LazyModule = bin::from_lib::regist_lazy();
    static ABOUT_LIB: LazyModule = bin::about_lib::regist_lazy();
    static CONSOLE_LIB: LazyModule = bin::console_lib::regist_lazy();
}

fn regist_all_info() -> BTreeMap<&'static str, BTreeMap<&'static str, BuiltinInfo>> {
    let mut libs_info = BTreeMap::new();
    libs_info.insert("", bin::top::regist_info());
    libs_info.insert("", bin::se_lib::regist_info()); //regist to top
    libs_info.insert("boolean", bin::boolean_lib::regist_info());
    libs_info.insert("string", bin::string_lib::regist_info());
    libs_info.insert("list", bin::list_lib::regist_info());
    libs_info.insert("set", bin::bset_lib::regist_info());
    libs_info.insert("map", bin::map_lib::regist_info());
    libs_info.insert("hmap", bin::hmap_lib::regist_info());
    libs_info.insert("time", bin::time_lib::regist_info());
    libs_info.insert("regex", bin::reg_lib::regist_info());
    libs_info.insert("math", bin::math_lib::regist_info());
    libs_info.insert("rand", bin::rand_lib::regist_info());
    libs_info.insert("fs", bin::fs_lib::regist_info());
    libs_info.insert("filesize", bin::filesize_lib::regist_info());
    libs_info.insert("from", bin::from_lib::regist_info());
    libs_info.insert("into", bin::into_lib::regist_info());
    libs_info.insert("sys", bin::sys_lib::regist_info());
    libs_info.insert("ui", bin::ui_lib::regist_info());
    libs_info.insert("console", bin::console_lib::regist_info());
    libs_info.insert("log", bin::log_lib::regist_info());
    libs_info.insert("about", bin::about_lib::regist_info());
    libs_info
}
/// lazy load builtin.
/// note: this always clone builtin
pub fn get_builtin_optimized(lib_name: &str, fn_name: &str) -> Option<BuiltinFunc> {
    match lib_name {
        // "Math" => MATH_LIB.with(|m| m.borrow().get(function).cloned()),
        "" => TOP_LIB.with(|m| m.borrow().get(fn_name).cloned()),
        "boolean" => BOOL_LIB.with(|m| m.borrow().get(fn_name).cloned()),
        "string" => STRING_LIB.with(|m| m.get_function(fn_name)),
        "list" => LIST_LIB.with(|m| m.get_function(fn_name)),
        "set" => BSET_LIB.with(|m| m.get_function(fn_name)),
        "map" => MAP_LIB.with(|m| m.get_function(fn_name)),
        "hmap" => HMAP_LIB.with(|m| m.get_function(fn_name)),
        "time" => TIME_LIB.with(|m| m.get_function(fn_name)),
        "regex" => REGEX_LIB.with(|m| m.get_function(fn_name)),
        "math" => MATH_LIB.with(|m| m.get_function(fn_name)),
        "rand" => RAND_LIB.with(|m| m.get_function(fn_name)),
        "log" => LOG_LIB.with(|m| m.get_function(fn_name)),
        "fs" => FS_LIB.with(|m| m.get_function(fn_name)),
        "ui" => UI_LIB.with(|m| m.get_function(fn_name)),
        "into" => INTO_LIB.with(|m| m.get_function(fn_name)),
        "sys" => SYS_LIB.with(|m| m.get_function(fn_name)),
        "filesize" => FILESIZE_LIB.with(|m| m.get_function(fn_name)),
        "from" => FROM_LIB.with(|m| m.get_function(fn_name)),
        "about" => ABOUT_LIB.with(|m| m.get_function(fn_name)),
        "console" => CONSOLE_LIB.with(|m| m.get_function(fn_name)),

        _ => None,
    }
}
// pub fn get_builtin_tips() -> HashSet<String> {
//     let mut tips = HashSet::new();
//     LIBS_INFO.with(|h| {
//         h.iter().for_each(|(lib, funcs)| {
//             if lib.is_empty() {
//                 for (name, info) in funcs {
//                     tips.insert(format!("{}  {}", name, info.hint));
//                 }
//             } else {
//                 for (name, info) in funcs {
//                     tips.insert(format!("{}.{}  {}", lib, name, info.hint));
//                 }
//             }
//         })
//     });
//     tips
// }

pub fn is_lib(name: &str) -> bool {
    LIBS_INFO.with(|h| h.contains_key(name))
}

pub fn get_lib_completions(prefix: &str) -> Option<Vec<&str>> {
    if prefix.is_empty() || !prefix.is_ascii() {
        return None;
    }
    let top = TOP_LIB.with(|h| {
        h.borrow()
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    });
    if !top.is_empty() {
        return Some(top);
    }
    let lib = LIBS_INFO.with(|h| {
        h.iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    });
    if !lib.is_empty() {
        return Some(lib);
    }
    None
}

/// 类型名称
fn get_belong_lib_name(exp: &Expression) -> Option<Cow<'static, str>> {
    match exp {
        Expression::List(_) | Expression::Range(..) => Some("list".into()),
        Expression::BSet(_) => Some("set".into()),
        Expression::Map(_) => Some("map".into()),
        Expression::HMap(_) => Some("hmap".into()),
        Expression::String(_) | Expression::StringTemplate(_) | Expression::Bytes(_) => {
            Some("string".into())
        }
        Expression::Integer(_) | Expression::Float(_) => Some("math".into()),
        Expression::DateTime(_) => Some("time".into()),
        Expression::Boolean(_) => Some("boolean".into()),
        Expression::Regex(_) => Some("regex".into()),
        Expression::FileSize(_) => Some("filesize".into()),
        _ => None,
    }
}

pub fn get_builtin_via_expr(expr: &Expression, fn_name: &str) -> Option<BuiltinFunc> {
    match expr {
        Expression::Blank => get_builtin_optimized("", fn_name),
        Expression::Symbol(x) => get_builtin_optimized(x.as_ref(), fn_name),
        other => {
            get_belong_lib_name(other).and_then(|x| get_builtin_optimized(x.as_ref(), fn_name))
        }
    }
}

pub fn exec_self_expand_lib(
    fn_name: &str,
    args: &[Expression],
    env: &mut Environment,
    state: &mut State,
    ctx: &Expression,
) -> Result<Option<Expression>, RuntimeError> {
    SE_LIB.with_borrow(|s| {
        if let Some(selib) = s.get(fn_name) {
            let result = selib(args, env, state, ctx)?;
            return Ok(Some(result));
        }
        return Ok(None);
    })
}
