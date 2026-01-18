use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::libs::BuiltinFunc;

// 函数工厂类型
type FunctionFactory = Box<dyn Fn() -> Rc<BuiltinFunc>>;

// 懒加载模块结构
pub struct LazyModule {
    functions: RefCell<HashMap<String, FunctionFactory>>,
    cache: RefCell<HashMap<String, Rc<BuiltinFunc>>>,
}

impl LazyModule {
    pub fn new() -> Self {
        Self {
            functions: RefCell::new(HashMap::new()),
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn register<F>(&self, name: &str, factory: F)
    where
        F: Fn() -> Rc<BuiltinFunc> + 'static,
    {
        self.functions
            .borrow_mut()
            .insert(name.to_string(), Box::new(factory));
    }

    pub fn get_function(&self, name: &str) -> Option<Rc<BuiltinFunc>> {
        // 先检查缓存
        if let Some(cached) = self.cache.borrow().get(name) {
            return Some(cached.clone());
        }

        // 懒加载函数
        self.functions.borrow().get(name).map(|factory| {
            let func = factory();
            self.cache
                .borrow_mut()
                .insert(name.to_string(), func.clone());
            func
        })
    }
}
