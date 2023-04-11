use dashmap::{
    DashMap,
};

use anyhow::{
    Result,
    bail
};

use crate::http::{
    ScopeProvide,
    DispatchScope
};

use std::rc::Rc;

#[derive(Debug)]
pub enum DispatchScopeValue {
    String(String),
    VecU8(Vec<u8>),
    I32(i32),
    Bool(bool),
}

trait AccessDispatchScopeValue: Sized {
    fn unwrap(wrapped_value: &DispatchScopeValue) -> Result<&Self>;
    fn move_out(wrapped_value: DispatchScopeValue) -> Result<Self>;
}

macro_rules! unwrap_scope_value {
    ($enum_const_fn:path, $($wrapped_value:tt)*) => {
            if let $enum_const_fn(value) = $($wrapped_value)* {
                Ok(value)
            } else {
                bail!("The DispatchScopeValue is of different type. Correct type is: {:?}", $($wrapped_value)*)
            }
    }
}

impl AccessDispatchScopeValue for String {
    fn unwrap(wrapped_value: &DispatchScopeValue) -> Result<&Self> {
        unwrap_scope_value!(DispatchScopeValue::String, wrapped_value)
    }

    fn move_out(wrapped_value: DispatchScopeValue) -> Result<Self> {
        unwrap_scope_value!(DispatchScopeValue::String, wrapped_value)
    }
}

impl AccessDispatchScopeValue for Vec<u8> {
    fn unwrap(wrapped_value: &DispatchScopeValue) -> Result<&Self> {
        unwrap_scope_value!(DispatchScopeValue::VecU8, wrapped_value)
    }

    fn move_out(wrapped_value: DispatchScopeValue) -> Result<Self> {
        unwrap_scope_value!(DispatchScopeValue::VecU8, wrapped_value)
    }
}

impl AccessDispatchScopeValue for i32 {
    fn unwrap(wrapped_value: &DispatchScopeValue) -> Result<&Self> {
        unwrap_scope_value!(DispatchScopeValue::I32, wrapped_value)
    }

    fn move_out(wrapped_value: DispatchScopeValue) -> Result<Self> {
        unwrap_scope_value!(DispatchScopeValue::I32, wrapped_value)
    }
}

impl AccessDispatchScopeValue for bool {
    fn unwrap(wrapped_value: &DispatchScopeValue) -> Result<&Self> {
        unwrap_scope_value!(DispatchScopeValue::Bool, wrapped_value)
    }

    fn move_out(wrapped_value: DispatchScopeValue) -> Result<Self> {
        unwrap_scope_value!(DispatchScopeValue::Bool, wrapped_value)
    }
}

struct ServerDispatchScope {
    map: DashMap<&'static str, DispatchScopeValue>,
}

impl ServerDispatchScope {
    pub fn new() -> Self {
        ServerDispatchScope {
            map: DashMap::with_capacity(1)
        }
    }

    pub fn get_value<T: AccessDispatchScopeValue + Clone>(&self, key: &'static str) -> Result<T> {
        if let Some(val_ref) = self.map.get(key) {
            let value = T::unwrap(val_ref.value())?;
            Ok((*value).clone())
        } else {
            bail!("No value with key '{}' in this scope", key)
        }
    }

    pub fn set_value<R: AccessDispatchScopeValue>(&self, key: &'static str, value: DispatchScopeValue) -> Option<R> {
        let mut ret_val: Option<R> = None;
        if let Some(val_ref) = self.map.insert(key, value) {
            match R::move_out(val_ref) {
                Ok(value) => ret_val = Some(value),
                Err(err) => {
                    log::warn!("Could not move existing scope value with key '{}' out of the scope.\
                                Possible reason: Type of replaced value differs from type of new value.\
                                Err: {}", key, err);
                }
            }
        }

        ret_val
    }
}

impl DispatchScope for ServerDispatchScope {

    fn contains_key(&self, key: &'static str) -> bool  {
        self.map.contains_key(key)
    }

    fn get_bool(&self, key: &'static str) -> Result<bool> {
        self.get_value::<bool>(key)
    }

    fn get_string(&self, key: &'static str) -> Result<String>{
        self.get_value::<String>(key)
    }

    fn get_vec_u8(&self, key: &'static str) -> Result<Vec<u8>> { self.get_value::<Vec<u8>>(key) }

    fn get_i32(&self, key: &'static str) -> Result<i32> {
        self.get_value::<i32>(key)
    }

    fn set_bool(&self, key: &'static str, value: &bool) -> Option<bool> {
        self.set_value(key, DispatchScopeValue::Bool(value.clone()))
    }

    fn set_string(&self, key: &'static str, value: &str) -> Option<String> {
        self.set_value(key, DispatchScopeValue::String(value.to_string()))
    }

    fn set_vec_u8(&self, key: &'static str, value: Vec<u8>) -> Option<Vec<u8>> {
        self.set_value(key, DispatchScopeValue::VecU8(value.clone()))
    }

    fn set_i32(&self, key: &'static str, value: &i32) -> Option<i32> {
        self.set_value(key, DispatchScopeValue::I32(value.clone()))
    }
}

#[derive(Clone)]
pub struct ServerScopeProvide {
    scope: Option<Rc<ServerDispatchScope>>,
}

impl ServerScopeProvide {
    pub fn new() -> Self {
        ServerScopeProvide {
            scope: None
        }
    }
}

impl ScopeProvide for ServerScopeProvide {
    fn create_new_scope(&mut self) -> Rc<dyn DispatchScope> {
        /*
        if let Some(rc_scope) = self.scope.as_mut() {
            drop(rc_scope);
        };
        */

        if let Some(rc_scope) = self.scope.as_mut() {
            let strong_cnt = Rc::strong_count(rc_scope);
            if strong_cnt > 1 {
                panic!("Attempt to create a new dispatch scope while the old scope is already used.\
                        Rc::strong_count() of existing scope is {}", strong_cnt);
            };
        };

        let ret_val = Rc::new(ServerDispatchScope::new());
        self.scope = Some(ret_val.clone());
        ret_val
    }

    fn get_scope(&self) -> Rc<dyn DispatchScope> {
        match self.scope.as_ref() {
            None => {
                panic!("You need to call ServerScopeProvide::create_new_scope() before you can use get_scope() function");
            }
            Some(ret_val) => ret_val.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_dispatch_scope_value_multiple() {
        let mut scope_provide = ServerScopeProvide::new();
        let scope = scope_provide.create_new_scope();

        scope.set_bool("bool-value", &true);
        scope.set_string("string-value", "Some string");
        scope.set_vec_u8("vec-u8-value", vec![1, 2, 3]);
        scope.set_i32("i32-value", &123);

        assert_eq!(scope.get_bool("bool-value").unwrap(), true);
        assert_eq!(scope.get_string("string-value").unwrap(), "Some string");
        assert_eq!(scope.get_vec_u8("vec-u8-value").unwrap(), vec![1, 2, 3]);
        assert_eq!(scope.get_i32("i32-value").unwrap(), 123);

        assert_eq!(scope.contains_key("i32-value"), true);
        assert_eq!(scope.contains_key("nonsense key"), false);
    }

    #[test]
    #[should_panic]
    fn test_multiple_scopes() {
        let mut scope_provide = ServerScopeProvide::new();
        let scope = scope_provide.create_new_scope();

        scope.set_bool("bool-value", &true);

        let scope_2 = scope_provide.create_new_scope();
        assert_eq!(scope.get_bool("bool-value").unwrap(), true);

        scope_2.set_bool("bool-value", &false);
        // we should not arrive here because having multiple scopes is not allowed
        assert_eq!(scope_2.get_bool("bool-value").unwrap(), false);
    }
}

