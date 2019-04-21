#[derive(PartialEq, Debug)]
pub struct Class {
    pub minor_version: u16,
    pub major_version: u16,
    pub constants: Vec<Constant>,
    pub flags: ClassFlags,
    pub this_class: ConstantIndex,
    pub super_class: ConstantIndex,
    pub interfaces: Vec<ConstantIndex>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct ConstantIndex(pub u16);

#[derive(PartialEq, Eq, Debug)]
pub struct MethodIndex(pub u16);

#[derive(PartialEq, Debug)]
pub enum Constant {
    Utf8(String),
    Integer(u32),
    Float(f32),
    Long(u64),
    Double(f64),
    ClassRef(ConstantIndex),
    StringRef(ConstantIndex),
    FieldRef{class:ConstantIndex, name_and_type:ConstantIndex},
    MethodRef{class:ConstantIndex, name_and_type:ConstantIndex},
    InterfaceMethodRef{class:ConstantIndex, name_and_type:ConstantIndex},
    NameAndTypeRef{name:ConstantIndex, descriptor:ConstantIndex},
    MethodHandle(MethodHandleType),
    MethodType(ConstantIndex),
    InvokeDynamicInfo{bootstrap_method_attr:MethodIndex, name_and_type:ConstantIndex},
    Dummy, // Necessary to fake Long and Double taking up two slots
}

impl Constant {
    pub fn get_tag(self) -> Option<u8> {
        match self {
            Constant::Utf8(_) => Some(1),
            Constant::Integer(_) => Some(3),
            Constant::Float(_) => Some(4),
            Constant::Long(_) => Some(5),
            Constant::Double(_) => Some(6),
            Constant::ClassRef(_) => Some(7),
            Constant::StringRef(_) => Some(8),
            Constant::FieldRef{..} => Some(9),
            Constant::MethodRef{..} => Some(10),
            Constant::InterfaceMethodRef{..} => Some(11),
            Constant::NameAndTypeRef{..} => Some(12),
            Constant::MethodHandle(_) => Some(15),
            Constant::MethodType(_) => Some(16),
            Constant::InvokeDynamicInfo{..} => Some(18),
            Constant::Dummy => None,
        }
    }
}

bitflags! {
    pub struct ClassFlags: u16 {
        const PUBLIC     = 0x0001;
        const FINAL      = 0x0010;
        const SUPER      = 0x0020;
        const INTERFACE  = 0x0200;
        const ABSTRACT   = 0x0040;
        const SYNTHETIC  = 0x1000;
        const ANNOTATION = 0x2000;
        const ENUM       = 0x4000;
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Field {
    pub flags: FieldFlags,
    pub name: ConstantIndex,
    pub descriptor: ConstantIndex,
    pub attributes: Vec<Attribute>,
}

bitflags! {
    pub struct FieldFlags: u16 {
        const PUBLIC    = 0x0001;
        const PRIVATE   = 0x0002;
        const PROTECTED = 0x0004;
        const STATIC    = 0x0008;
        const FINAL     = 0x0010;
        const VOLATILE  = 0x0040;
        const TRANSIENT = 0x0080;
        const SYNTHETIC = 0x1000;
        const ENUM      = 0x4000;
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Method {
    pub flags: MethodFlags,
    pub name: ConstantIndex,
    pub descriptor: ConstantIndex,
    pub attributes: Vec<Attribute>,
}

bitflags! {
    pub struct MethodFlags: u16 {
        const PUBLIC       = 0x0001;
        const PRIVATE      = 0x0002;
        const PROTECTED    = 0x0004;
        const STATIC       = 0x0008;
        const FINAL        = 0x0010;
        const SYNCHRONIZED = 0x0020;
        const BRIDGE       = 0x0040;
        const VARARGS      = 0x0080;
        const NATIVE       = 0x0100;
        const ABSTRACT     = 0x0400;
        const STRICT       = 0x0800;
        const SYNTHETIC    = 0x1000;
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Attribute {
    pub name: ConstantIndex,
    pub contents: Vec<u8>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum MethodHandleType {
    GetField(ConstantIndex),
    GetStatic(ConstantIndex),
    PutField(ConstantIndex),
    PutStatic(ConstantIndex),
    InvokeVirtual(ConstantIndex),
    InvokeStatic(ConstantIndex),
    InvokeSpecial(ConstantIndex),
    NewInvokeSpecial(ConstantIndex),
    InvokeInterface(ConstantIndex),
}
