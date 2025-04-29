use std::{collections::HashMap, sync::LazyLock};

use crate::Expression;

mod init;

// pub use init::get_builtin;
// pub use init::init;

static BUILTIN: LazyLock<HashMap<String, Expression>> = LazyLock::new(|| init::get_module_map());
pub fn get_builtin(name: &str) -> Option<&Expression> {
    BUILTIN.get(name)
}
