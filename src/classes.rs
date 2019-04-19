mod classes {
    pub struct Class {
        pub minor_version: u16,
        pub major_version: u16,
        pub constants: Vec<Constant>,
        access_flags: u16,
        pub this_class: ConstantRef,
        pub super_class: ConstantRef,
        pub interfaces: Vec<ConstantRef>,
        pub fields: Vec<Field>,
        pub methods: Vec<Method>,
        pub attributes: Vec<Attribute>,
    }

    pub struct ConstantRef(u16);

    pub enum Constant {
        Dummy,
    }

    pub struct Field {
    }

    pub struct Method {
    }

    pub struct Attribute {
    }
}
