use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use crate::{Expression, modules::bin::pprint::pprint_hmap};

mod bin;
// use std::sync::RwLock;
pub use bin::fs_module::join_current_path_with_home;
pub use bin::pprint::pretty_printer;
pub use bin::time_module::parse_time;

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
    for (key, item) in get_builtin_map().into_iter() {
        match item {
            Expression::HMap(m) => {
                let table = pprint_hmap(m.as_ref());
                tips.insert(format!("{key}. \n{table}"));
                for (k, v) in m.iter() {
                    // dbg!(&k, &v.to_string(), v.type_name());
                    match v {
                        Expression::Builtin(b) => {
                            // dbg!(&b.name, &b.hint);
                            match b.hint.is_empty() {
                                true => tips.insert(format!("{key}.{k}()")),
                                false => tips.insert(format!("{}.{} {}", key, k, b.hint)),
                            };
                        }
                        Expression::HMap(mm) => {
                            for (mk, _) in mm.iter() {
                                tips.insert(format!("{key}.{k}.{mk}"));
                            }
                        }
                        _ => {
                            tips.insert(format!("{key}.{k}"));
                        }
                    }
                }
            }
            Expression::Map(m) => {
                // tips.insert(format!("{}. \n{}", key, item));
                for (k, _) in m.iter() {
                    tips.insert(format!("{key}.{k}"));
                }
            }
            _ => {
                tips.insert(key.to_owned());
            }
        }
    }
    tips
}
pub fn get_builtin_symbos() -> HashSet<String> {
    let mut tips: HashSet<String> = HashSet::new();

    for (key, item) in get_builtin_map().iter() {
        tips.insert(key.to_owned());
        match item {
            Expression::HMap(m) => {
                for (k, v) in m.iter() {
                    tips.insert(k.to_owned());
                    if let Expression::HMap(mm) = v {
                        for (mk, _) in mm.iter() {
                            tips.insert(mk.to_owned());
                        }
                    }
                }
            }
            Expression::Map(m) => {
                for (k, _) in m.iter() {
                    tips.insert(k.to_owned());
                }
            }
            _ => {}
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
