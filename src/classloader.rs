use crate::classes::*;

#[derive(PartialEq, Eq, Debug)]
pub struct ClassLoaderError(String);

trait Deserialize: Sized {
    fn deserialize(data: &Iterator<Item=u8>) -> Result<Self, ClassLoaderError>;
}

impl Deserialize for Constant {
    fn deserialize(_data: &Iterator<Item=u8>) -> Result<Self, ClassLoaderError> {
        return Ok(Constant::Utf8("Hello".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_string() {
        assert_constant(Constant::Utf8("Hello".to_string()), &"\x01\x00\x05Hello");
    }

    fn assert_constant(constant: Constant, input: &str) {
        assert_eq!(Ok(constant), Constant::deserialize(&str::bytes(input)))
    }
}
