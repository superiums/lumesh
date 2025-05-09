use std::{collections::HashMap, sync::LazyLock};

use crate::Expression;

mod init;
// use std::sync::RwLock;

struct UnsafeStatic<T> {
    inner: T,
}
unsafe impl<T> Sync for UnsafeStatic<T> {} // 手动标记为 Sync（单线程安全）

static BUILTIN: UnsafeStatic<LazyLock<HashMap<String, Expression>>> = UnsafeStatic {
    inner: LazyLock::new(|| init::get_module_map()),
};

pub fn get_builtin(name: &str) -> Option<&Expression> {
    BUILTIN.inner.get(name)
}

// use std::sync::OnceLock;

// static BUILTIN: OnceLock<HashMap<String, Expression>> = OnceLock::new();

// pub fn get_builtin(name: &str) -> Option<&'static Expression> {
//     BUILTIN.get_or_init(|| init::get_module_map()).get(name)
// }

// use std::cell::RefCell;
// use std::collections::HashMap;
// use std::rc::Rc;

// struct UnsafeStatic<T> {
//     inner: T,
// }
// unsafe impl<T> Sync for UnsafeStatic<T> {} // 手动标记为 Sync（单线程安全）

// static BUILTIN: UnsafeStatic<RefCell<HashMap<String, Expression>>> = UnsafeStatic {
//     inner: RefCell::new(HashMap::new()),
// };

// pub fn init_builtin() {
//     *BUILTIN.inner.as_ref_mut() = init::get_module_map(); // 初始化
// }

// pub fn get_builtin(name: &str) -> Option<Expression> {
//     BUILTIN.inner.as_ref().get(name).cloned()
// }
