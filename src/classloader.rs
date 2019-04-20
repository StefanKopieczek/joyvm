use crate::classes;

pub struct ClassLoaderError(String);

trait Deserialize: Sized {
    fn deserialize(data: &mut Iterator<Item=u8>) -> Result<Self, ClassLoaderError>;
}

impl Deserialize for classes::Constant {
    fn deserialize(_data: &mut Iterator<Item=u8>) -> Result<Self, ClassLoaderError> {
        return Err(ClassLoaderError("Not yet implemented!".to_string()));
    }
}
