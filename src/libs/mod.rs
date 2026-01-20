mod bin;
mod helper;
mod lazy_module;
mod pprint;
use crate::{Environment, Expression, RuntimeError, libs::lazy_module::LazyModule};
pub use bin::top::regist_info;
pub use pprint::pretty_printer;
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    rc::Rc,
    sync::LazyLock,
};

pub struct BuiltinInfo {
    pub descr: &'static str,
    pub hint: &'static str,
}
pub type BuiltinFunc =
    fn(&[Expression], &mut Environment, contex: &Expression) -> Result<Expression, RuntimeError>;

// 对不同模块采用不同策略
thread_local! {
    // 帮助信息，初次使用时加载
    pub static LIBS_INFO: LazyLock<BTreeMap<&'static str, BTreeMap<&'static str,BuiltinInfo>>> =LazyLock::new(regist_all_info);

    // 热模块/小型模块：完全预加载
    // static MATH_MODULE: RefCell<HashMap<String, Expression>> = RefCell::new({
    //     math_module::get_all_functions() // 加载所有函数
    // });
    static TOP_MODULE: RefCell<HashMap<&'static str, Rc<BuiltinFunc>>> = RefCell::new({
        bin::top::regist_all()
    });
    static BOOL_MODULE: RefCell<HashMap<&'static str, Rc<BuiltinFunc>>> = RefCell::new({
        bin::boolean_lib::regist_all()
    });

    // 中型模块：模块级懒加载
    // static FS_MODULE: RefCell<Option<Expression>> = RefCell::new(None);


    // 大型模块：函数级懒加载
    static STRING_MODULE: LazyModule = bin::string_lib::regist_lazy();
    static LIST_MODULE: LazyModule = bin::list_lib::regist_lazy();
    static MAP_MODULE: LazyModule = bin::map_lib::regist_lazy();
    static TIME_MODULE: LazyModule = bin::time_lib::regist_lazy();
    static REGEX_MODULE: LazyModule = bin::reg_lib::regist_lazy();
    static MATH_MODULE: LazyModule = bin::math_lib::regist_lazy();
    static RAND_MODULE: LazyModule = bin::rand_lib::regist_lazy();
    static LOG_MODULE: LazyModule = bin::log_lib::regist_lazy();
    static FS_MODULE: LazyModule = bin::fs_lib::regist_lazy();
    static UI_MODULE: LazyModule = bin::ui_lib::regist_lazy();
    static INTO_MODULE: LazyModule = bin::into_lib::regist_lazy();
    static SYS_MODULE: LazyModule = bin::sys_lib::regist_lazy();
    static FILESIZE_MODULE: LazyModule = bin::filesize_lib::regist_lazy();
    static FROM_MODULE: LazyModule = bin::from_lib::regist_lazy();
    static ABOUT_MODULE: LazyModule = bin::about_lib::regist_lazy();
}

fn regist_all_info() -> BTreeMap<&'static str, BTreeMap<&'static str, BuiltinInfo>> {
    let mut libs_info = BTreeMap::new();
    libs_info.insert("", bin::top::regist_info());
    libs_info.insert("string", bin::string_lib::regist_info());
    libs_info.insert("boolean", bin::boolean_lib::regist_info());
    libs_info.insert("list", bin::list_lib::regist_info());
    libs_info.insert("map", bin::map_lib::regist_info());
    libs_info.insert("time", bin::time_lib::regist_info());
    libs_info.insert("regex", bin::reg_lib::regist_info());
    libs_info.insert("math", bin::math_lib::regist_info());
    libs_info.insert("rand", bin::rand_lib::regist_info());
    libs_info.insert("log", bin::log_lib::regist_info());
    libs_info.insert("fs", bin::fs_lib::regist_info());
    libs_info.insert("ui", bin::ui_lib::regist_info());
    libs_info.insert("into", bin::into_lib::regist_info());
    libs_info.insert("sys", bin::sys_lib::regist_info());
    libs_info.insert("filesize", bin::filesize_lib::regist_info());
    libs_info.insert("from", bin::from_lib::regist_info());
    libs_info.insert("about", bin::about_lib::regist_info());
    libs_info
}
/// lazy load builtin.
/// note: this always clone builtin
pub fn get_builtin_optimized(lib_name: &str, fn_name: &str) -> Option<Rc<BuiltinFunc>> {
    match lib_name {
        // "Math" => MATH_MODULE.with(|m| m.borrow().get(function).cloned()),
        "" => TOP_MODULE.with(|m| m.borrow().get(fn_name).cloned()),
        "boolean" => BOOL_MODULE.with(|m| m.borrow().get(fn_name).cloned()),
        "string" => STRING_MODULE.with(|m| m.get_function(fn_name)),
        "list" => LIST_MODULE.with(|m| m.get_function(fn_name)),
        "map" => MAP_MODULE.with(|m| m.get_function(fn_name)),
        "time" => TIME_MODULE.with(|m| m.get_function(fn_name)),
        "regex" => REGEX_MODULE.with(|m| m.get_function(fn_name)),
        "math" => MATH_MODULE.with(|m| m.get_function(fn_name)),
        "rand" => RAND_MODULE.with(|m| m.get_function(fn_name)),
        "log" => LOG_MODULE.with(|m| m.get_function(fn_name)),
        "fs" => FS_MODULE.with(|m| m.get_function(fn_name)),
        "ui" => UI_MODULE.with(|m| m.get_function(fn_name)),
        "into" => INTO_MODULE.with(|m| m.get_function(fn_name)),
        "sys" => SYS_MODULE.with(|m| m.get_function(fn_name)),
        "filesize" => FILESIZE_MODULE.with(|m| m.get_function(fn_name)),
        "from" => FROM_MODULE.with(|m| m.get_function(fn_name)),
        "about" => ABOUT_MODULE.with(|m| m.get_function(fn_name)),
        // filesize from
        // "Fs" => FS_MODULE.with(|m| {
        //     if m.borrow().is_none() {
        //         *m.borrow_mut() = Some(fs_module::get());
        //     }
        //     m.borrow().as_ref().and_then(|mod_expr| {
        //         if let Expression::HMap(map) = mod_expr {
        //             map.get(function).cloned()
        //         } else {
        //             None
        //         }
        //     })
        // }),
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
    let top = TOP_MODULE.with(|h| {
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
        Expression::Map(_) | Expression::HMap(_) => Some("map".into()),
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
pub fn get_builtin_via_expr(expr: &Expression, fn_name: &str) -> Option<Rc<BuiltinFunc>> {
    match expr {
        Expression::Blank => get_builtin_optimized("", fn_name),
        Expression::Symbol(x) => get_builtin_optimized(x.as_ref(), fn_name),
        other => {
            get_belong_lib_name(other).and_then(|x| get_builtin_optimized(x.as_ref(), fn_name))
        }
    }
}

// pub fn eval_builtin_optimized(
//     lib_name: &str,
//     fn_name: &str,
//     arg_base: &Expression,
//     args: &[Expression],
//     env: &mut Environment,
//     context: &Expression,
//     depth: usize,
// ) -> (bool, Result<Expression, RuntimeError>) {
//     let fo = match lib_name {
//         // "Math" => MATH_MODULE.with(|m| m.borrow().get(function).cloned()),
//         "String" => STRING_MODULE.with(|m| m.get_function(fn_name)),
//         "" => TOP_MODULE.with(|m| m.borrow().get(fn_name).cloned()),
//         _ => None,
//     };
//     match fo.as_ref() {
//         Some(f) => {
//             let result = f(args, env, context);
//             (true, result)
//         }
//         _ => (false, Ok(Expression::None)),
//     }
// }

// 修改 get_module_map() 使用懒加载模块
// pub fn get_module_map() -> HashMap<String, Expression> {
//     hash_map! {
//         // 其他模块保持不变
//         // String::from("Log") => log_module::get(),
//         // String::from("Math") => math_module::get(),

//         // String 模块使用懒加载包装器
//         String::from("String") => Expression::from(LazyModuleWrapper {
//             getter: get_string_function,
//         }),
//     }
// }

// // 懒加载模块包装器
// struct LazyModuleWrapper {
//     getter: fn(&str) -> Option<Expression>,
// }

// impl LazyModuleWrapper {
//     fn get(&self, name: &str) -> Option<Expression> {
//         (self.getter)(name)
//     }
// }
