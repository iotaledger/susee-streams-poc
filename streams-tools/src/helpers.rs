use rand::Rng;
use anyhow::Result;

pub fn create_psk_seed() -> [u8; 32] {
    rand::thread_rng().gen::<[u8; 32]>()
}

pub fn get_channel_id_from_link(streams_link: &str) -> Option<String> {
    let parts = streams_link.split(":");
    let mut ret_val = None;
    for part in parts {
        ret_val = Some(String::from(part));
        break;
    }
    ret_val
}

// -------------------------------------------------------------------------------------
// SerializationCallbackCloneBox and SerializationCallbackRefToClosure are used to handle
// closures
//          move |key: String, data_to_serialize: Vec<u8>| -> Result<usize> {
//              ....
//          }
//
// as function arguments, fields and e.g. to provide them as optional values
// using Option<SerializationCallbackRefToClosure>.
//
// Please note that in most cases 'move' needs to be used to implement the closure.
// This will move all values captured in the closure into the closure instead of using
// references. References will not work when the scope that originally create the closure
// does not exist anymore when the closure is evaluated.
//
// The recipe used here has been taken from
// https://users.rust-lang.org/t/how-to-clone-a-boxed-closure/31035/25
//
// Nice article about closures in Rust:
// https://stevedonovan.github.io/rustifications/2018/08/18/rust-closures-are-hard.html
// -------------------------------------------------------------------------------------

pub trait SerializationCallbackCloneBox<PrimaryKeyType: Clone>: FnOnce(PrimaryKeyType, Vec<u8>) -> Result<usize> {
    fn clone_box(&self) -> Box<dyn SerializationCallbackCloneBox<PrimaryKeyType>>;
}

impl<T, PrimaryKeyType> SerializationCallbackCloneBox<PrimaryKeyType> for T
    where
        T: 'static + FnOnce(PrimaryKeyType, Vec<u8>) -> Result<usize> + Clone,
        PrimaryKeyType: Clone,
{
    fn clone_box(&self) -> Box<dyn SerializationCallbackCloneBox<PrimaryKeyType>> {
        Box::new(self.clone())
    }
}

impl<PrimaryKeyType: Clone> Clone for Box<dyn SerializationCallbackCloneBox<PrimaryKeyType>> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

// This can be used to store closures that will serialize a Streams user state
// String - pub streams_channel_id
// Vec<u8> - pub streams_user_state
// -> Result<usize> - number of rows or bytes
pub type SerializationCallbackRefToClosureString = Box<dyn SerializationCallbackCloneBox<String>>;
pub type SerializationCallbackRefToClosureI64 = Box<dyn SerializationCallbackCloneBox<i64>>;

