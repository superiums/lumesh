use super::Expression;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

thread_local! {
    static ALIAS_MAP: RefCell<HashMap<String, Expression>> = RefCell::new(HashMap::new());
}

pub fn set_alias(name: String, expression: Expression) {
    ALIAS_MAP.with(|map| {
        map.borrow_mut().insert(name, expression);
    });
}

pub fn get_alias(name: &str) -> Option<Expression> {
    ALIAS_MAP.with(|map| map.borrow().get(name).cloned())
}

// pub fn get_alias_tips() -> HashSet<String> {
//     ALIAS_MAP.with(|map| map.borrow().keys().map(|k| k.to_string()).collect())
// }
// pub fn get_alias_completion(prefix: &str) -> Vec<String> {
//     ALIAS_MAP.with(|map| {
//         map.borrow()
//             .keys()
//             .filter(|m| m.starts_with(prefix))
//             .map(|k| k.to_string())
//             .collect::<Vec<_>>()
//     })
// }

// fn remove_alias(name: &str) {
//     ALIAS_MAP.with(|map| {
//         map.borrow_mut().remove(name);
//     });
// }
