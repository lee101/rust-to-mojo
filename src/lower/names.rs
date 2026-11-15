//! Identifier and primitive-type naming.

/// Mojo reserved words and soft keywords that a Rust identifier could collide
/// with. Colliding identifiers get a trailing underscore.
const MOJO_RESERVED: &[&str] = &[
    "def", "fn", "var", "struct", "trait", "comptime", "alias", "from", "import", "in", "is", "not", "and", "or",
    "pass", "raise", "raises", "try", "except", "finally", "with", "lambda", "class", "del", "yield", "global",
    "nonlocal", "elif", "async", "await", "assert", "None", "True", "False", "out", "mut", "ref", "deinit", "imm",
    "read", "owned", "associatedtype", "where", "match", "case", "print", "len", "range", "inout", "borrowed",
];

pub fn ident(rust: &str) -> String {
    let r = rust.strip_prefix("r#").unwrap_or(rust);
    if MOJO_RESERVED.contains(&r) {
        format!("{}_", r)
    } else {
        r.to_string()
    }
}

/// Rust primitive -> Mojo type name.
pub fn prim(name: &str) -> Option<&'static str> {
    Some(match name {
        "i8" => "Int8",
        "i16" => "Int16",
        "i32" => "Int32",
        "i64" => "Int64",
        "i128" => "Int128",
        "u8" => "UInt8",
        "u16" => "UInt16",
        "u32" => "UInt32",
        "u64" => "UInt64",
        "u128" => "UInt128",
        // Rust uses usize for lengths/indices; Mojo's idiomatic index type is
        // the signed, platform-width `Int` (what `len()` returns). See MAPPING.md.
        "usize" => "Int",
        "isize" => "Int",
        "f32" => "Float32",
        "f64" => "Float64",
        "bool" => "Bool",
        "char" => "Codepoint",
        "str" => "String",
        "String" => "String",
        _ => return None,
    })
}

pub fn is_float(name: &str) -> bool {
    matches!(name, "f32" | "f64")
}

pub fn is_int(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "isize"
    )
}

pub fn is_numeric(name: &str) -> bool {
    is_float(name) || is_int(name)
}

/// Rust trait used as a bound -> Mojo trait, if there is a sound counterpart.
pub fn std_trait(name: &str) -> Option<&'static str> {
    Some(match name {
        "Clone" => "Copyable",
        "Copy" => "ImplicitlyCopyable",
        "Debug" | "Display" => "Writable",
        "Default" => "Defaultable",
        "PartialEq" | "Eq" => "Equatable",
        "PartialOrd" | "Ord" => "Comparable",
        "Hash" => "Hashable",
        _ => return None,
    })
}

/// Rust traits that are erased in Mojo (marker or auto traits).
pub fn erased_trait(name: &str) -> bool {
    matches!(name, "Sized" | "Send" | "Sync" | "Unpin" | "UnwindSafe" | "RefUnwindSafe" | "Any")
}

/// std::ops traits: (trait, method, mojo dunder, reflected dunder)
pub fn op_trait(name: &str) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    Some(match name {
        "Add" => ("add", "__add__", Some("__radd__")),
        "Sub" => ("sub", "__sub__", Some("__rsub__")),
        "Mul" => ("mul", "__mul__", Some("__rmul__")),
        "Div" => ("div", "__truediv__", Some("__rtruediv__")),
        "Rem" => ("rem", "__mod__", Some("__rmod__")),
        "BitAnd" => ("bitand", "__and__", Some("__rand__")),
        "BitOr" => ("bitor", "__or__", Some("__ror__")),
        "BitXor" => ("bitxor", "__xor__", Some("__rxor__")),
        "Shl" => ("shl", "__lshift__", Some("__rlshift__")),
        "Shr" => ("shr", "__rshift__", Some("__rrshift__")),
        "Neg" => ("neg", "__neg__", None),
        "Not" => ("not", "__invert__", None),
        "AddAssign" => ("add_assign", "__iadd__", None),
        "SubAssign" => ("sub_assign", "__isub__", None),
        "MulAssign" => ("mul_assign", "__imul__", None),
        "DivAssign" => ("div_assign", "__itruediv__", None),
        "RemAssign" => ("rem_assign", "__imod__", None),
        "BitAndAssign" => ("bitand_assign", "__iand__", None),
        "BitOrAssign" => ("bitor_assign", "__ior__", None),
        "BitXorAssign" => ("bitxor_assign", "__ixor__", None),
        "ShlAssign" => ("shl_assign", "__ilshift__", None),
        "ShrAssign" => ("shr_assign", "__irshift__", None),
        _ => return None,
    })
}
