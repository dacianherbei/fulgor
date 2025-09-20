/// A minimal generic Scene struct.
/// Later this will contain primitives, lights, etc.
#[derive(Debug)]
pub struct Scene<T> {
    pub name: String,
    pub scale: T,
}

impl<T: Default> Scene<T> {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            scale: T::default(),
        }
    }
}
