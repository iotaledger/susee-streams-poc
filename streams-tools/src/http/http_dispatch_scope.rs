use anyhow::Result;
use std::rc::Rc;

pub trait DispatchScope {
    fn contains_key(&self, key: &'static str) -> bool;

    fn get_bool(&self, key: &'static str) -> Result<bool>;
    fn get_string(&self, key: &'static str) -> Result<String>;
    fn get_vec_u8(&self, key: &'static str) -> Result<Vec<u8>>;
    fn get_i32(&self, key: &'static str) -> Result<i32>;

    fn set_bool(&self, key: &'static str, value: &bool) -> Option<bool>;
    fn set_string(&self, key: &'static str, value: &str) -> Option<String>;
    fn set_vec_u8(&self, key: &'static str, value: Vec<u8>) -> Option<Vec<u8>>;
    fn set_i32(&self, key: &'static str, value: &i32) -> Option<i32>;
}

pub trait ScopeProvide {
    fn create_new_scope(&mut self) -> Rc<dyn DispatchScope>;
    fn get_scope(&self) -> Rc<dyn DispatchScope>;
}

pub trait ScopeConsume {
    fn set_scope(&mut self, scope: Rc<dyn DispatchScope>);
}