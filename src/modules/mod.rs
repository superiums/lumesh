use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use crate::Expression;

mod bin;
// use std::sync::RwLock;

struct UnsafeStatic<T> {
    inner: T,
}
unsafe impl<T> Sync for UnsafeStatic<T> {} // 手动标记为 Sync（单线程安全）

static BUILTIN: UnsafeStatic<LazyLock<HashMap<String, Expression>>> = UnsafeStatic {
    inner: LazyLock::new(bin::get_module_map),
};

pub fn get_builtin(name: &str) -> Option<&Expression> {
    BUILTIN.inner.get(name)
}
pub fn get_builtin_map() -> HashMap<String, Expression> {
    BUILTIN.inner.clone()
}
pub fn get_builtin_tips() -> HashSet<String> {
    let mut tips: HashSet<String> = HashSet::new();

    for (key, item) in get_builtin_map().iter() {
        match item {
            Expression::HMap(m) => {
                tips.insert(format!("{}.\n{}", key, item));
                for (k, _) in m.iter() {
                    tips.insert(format!("{}.{}", key, k));
                }
            }
            Expression::Map(m) => {
                // tips.insert(format!("{}. \n{}", key, item));
                for (k, _) in m.iter() {
                    tips.insert(format!("{}.{}", key, k));
                }
            }
            _ => {
                tips.insert(key.to_owned());
            }
        }
    }
    tips
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
