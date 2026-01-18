mod helper;
mod lazy_module;
mod pprint;
mod string_lib;
mod top_lib;
pub use pprint::pretty_printer;

use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::LazyLock};

use crate::{Environment, Expression, RuntimeError, libs::lazy_module::LazyModule};

// 对不同模块采用不同策略
thread_local! {
    // 热模块：完全预加载
    // static MATH_MODULE: RefCell<HashMap<String, Expression>> = RefCell::new({
    //     math_module::get_all_functions() // 加载所有函数
    // });
    static TOP_MODULE: RefCell<HashMap<&'static str, Rc<BuiltinFunc>>> = RefCell::new({
        top_lib::regist_all()  // 加载所有函数
    });

    // 大型模块：函数级懒加载
    static STRING_MODULE: LazyModule = string_lib::regist_lazy();

    // 中型模块：模块级懒加载
    // static FS_MODULE: RefCell<Option<Expression>> = RefCell::new(None);

    static LIBS_INFO: LazyLock<HashMap<&'static str, HashMap<&'static str,BuiltinInfo>>> =LazyLock::new(||regist_all_info());

}

pub struct BuiltinInfo {
    pub descr: &'static str,
    pub hint: &'static str,
}
pub type BuiltinFunc = fn(
    &Expression,
    &[Expression],
    &mut Environment,
    contex: &Expression,
    depth: usize,
) -> Result<Expression, RuntimeError>;

fn regist_all_info() -> HashMap<&'static str, HashMap<&'static str, BuiltinInfo>> {
    let mut libs_info = HashMap::with_capacity(17);
    libs_info.insert("String", string_lib::regist_info());
    libs_info
}

pub fn get_builtin_via_expr(expr: &Expression, fn_name: &str) -> Option<Rc<BuiltinFunc>> {
    match expr {
        Expression::Blank => get_builtin_optimized("", fn_name),
        Expression::Symbol(x) => get_builtin_optimized(x.as_ref(), fn_name),
        other => other
            .get_belong_lib_name()
            .and_then(|x| get_builtin_optimized(x.as_ref(), fn_name)),
    }
}
/// lazy load builtin.
/// note: this always clone builtin
pub fn get_builtin_optimized(lib_name: &str, fn_name: &str) -> Option<Rc<BuiltinFunc>> {
    match lib_name {
        // "Math" => MATH_MODULE.with(|m| m.borrow().get(function).cloned()),
        "" => TOP_MODULE.with(|m| m.borrow().get(fn_name).cloned()),
        "String" => STRING_MODULE.with(|m| m.get_function(fn_name)),
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

pub fn eval_builtin_optimized(
    lib_name: &str,
    fn_name: &str,
    arg_base: &Expression,
    args: &[Expression],
    env: &mut Environment,
    context: &Expression,
    depth: usize,
) -> (bool, Result<Expression, RuntimeError>) {
    let fo = match lib_name {
        // "Math" => MATH_MODULE.with(|m| m.borrow().get(function).cloned()),
        "String" => STRING_MODULE.with(|m| m.get_function(fn_name)),
        "" => TOP_MODULE.with(|m| m.borrow().get(fn_name).cloned()),
        _ => None,
    };
    match fo.as_ref() {
        Some(f) => {
            let result = f(arg_base, args, env, context, depth);
            (true, result)
        }
        _ => (false, Ok(Expression::None)),
    }
}

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
