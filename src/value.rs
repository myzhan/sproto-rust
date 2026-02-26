use std::collections::HashMap;
use std::fmt;

/// Dynamic value type for sproto, similar to `serde_json::Value`.
///
/// Represents any value that can be encoded/decoded by the sproto protocol.
#[derive(Clone, Debug)]
pub enum SprotoValue {
    /// Signed 64-bit integer.
    Integer(i64),
    /// Boolean value.
    Boolean(bool),
    /// UTF-8 string.
    Str(String),
    /// Raw binary data.
    Binary(Vec<u8>),
    /// IEEE 754 double-precision floating point.
    Double(f64),
    /// A struct (message) with named fields.
    Struct(HashMap<String, SprotoValue>),
    /// An ordered array of values.
    Array(Vec<SprotoValue>),
}

impl SprotoValue {
    /// Create a new empty struct value.
    pub fn new_struct() -> Self {
        SprotoValue::Struct(HashMap::new())
    }

    /// Helper to build a struct from key-value pairs.
    pub fn from_fields(fields: Vec<(&str, SprotoValue)>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in fields {
            map.insert(k.to_string(), v);
        }
        SprotoValue::Struct(map)
    }

    /// Get a field from a struct value, returns None if not a struct or field missing.
    pub fn get(&self, key: &str) -> Option<&SprotoValue> {
        match self {
            SprotoValue::Struct(map) => map.get(key),
            _ => None,
        }
    }

    /// Get as i64.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            SprotoValue::Integer(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as bool.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            SprotoValue::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as string slice.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SprotoValue::Str(v) => Some(v),
            _ => None,
        }
    }

    /// Get as f64.
    pub fn as_double(&self) -> Option<f64> {
        match self {
            SprotoValue::Double(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as struct map.
    pub fn as_struct(&self) -> Option<&HashMap<String, SprotoValue>> {
        match self {
            SprotoValue::Struct(map) => Some(map),
            _ => None,
        }
    }

    /// Get as array.
    pub fn as_array(&self) -> Option<&Vec<SprotoValue>> {
        match self {
            SprotoValue::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Get as binary slice.
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            SprotoValue::Binary(v) => Some(v),
            _ => None,
        }
    }

    /// Returns a short type description string.
    pub fn type_name(&self) -> &'static str {
        match self {
            SprotoValue::Integer(_) => "integer",
            SprotoValue::Boolean(_) => "boolean",
            SprotoValue::Str(_) => "string",
            SprotoValue::Binary(_) => "binary",
            SprotoValue::Double(_) => "double",
            SprotoValue::Struct(_) => "struct",
            SprotoValue::Array(_) => "array",
        }
    }
}

impl PartialEq for SprotoValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SprotoValue::Integer(a), SprotoValue::Integer(b)) => a == b,
            (SprotoValue::Boolean(a), SprotoValue::Boolean(b)) => a == b,
            (SprotoValue::Str(a), SprotoValue::Str(b)) => a == b,
            (SprotoValue::Binary(a), SprotoValue::Binary(b)) => a == b,
            (SprotoValue::Double(a), SprotoValue::Double(b)) => a.to_bits() == b.to_bits(),
            (SprotoValue::Struct(a), SprotoValue::Struct(b)) => a == b,
            (SprotoValue::Array(a), SprotoValue::Array(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for SprotoValue {}

impl fmt::Display for SprotoValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SprotoValue::Integer(v) => write!(f, "{}", v),
            SprotoValue::Boolean(v) => write!(f, "{}", v),
            SprotoValue::Str(v) => write!(f, "\"{}\"", v),
            SprotoValue::Binary(v) => write!(f, "<binary {} bytes>", v.len()),
            SprotoValue::Double(v) => write!(f, "{}", v),
            SprotoValue::Struct(map) => {
                write!(f, "{{ ")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            SprotoValue::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
        }
    }
}

// Conversion traits
impl From<i64> for SprotoValue {
    fn from(v: i64) -> Self {
        SprotoValue::Integer(v)
    }
}

impl From<i32> for SprotoValue {
    fn from(v: i32) -> Self {
        SprotoValue::Integer(v as i64)
    }
}

impl From<bool> for SprotoValue {
    fn from(v: bool) -> Self {
        SprotoValue::Boolean(v)
    }
}

impl From<String> for SprotoValue {
    fn from(v: String) -> Self {
        SprotoValue::Str(v)
    }
}

impl From<&str> for SprotoValue {
    fn from(v: &str) -> Self {
        SprotoValue::Str(v.to_string())
    }
}

impl From<f64> for SprotoValue {
    fn from(v: f64) -> Self {
        SprotoValue::Double(v)
    }
}

impl From<Vec<u8>> for SprotoValue {
    fn from(v: Vec<u8>) -> Self {
        SprotoValue::Binary(v)
    }
}

impl From<Vec<SprotoValue>> for SprotoValue {
    fn from(v: Vec<SprotoValue>) -> Self {
        SprotoValue::Array(v)
    }
}

impl From<HashMap<String, SprotoValue>> for SprotoValue {
    fn from(v: HashMap<String, SprotoValue>) -> Self {
        SprotoValue::Struct(v)
    }
}

impl From<Vec<i64>> for SprotoValue {
    fn from(v: Vec<i64>) -> Self {
        SprotoValue::Array(v.into_iter().map(SprotoValue::Integer).collect())
    }
}

impl From<Vec<f64>> for SprotoValue {
    fn from(v: Vec<f64>) -> Self {
        SprotoValue::Array(v.into_iter().map(SprotoValue::Double).collect())
    }
}

impl From<Vec<String>> for SprotoValue {
    fn from(v: Vec<String>) -> Self {
        SprotoValue::Array(v.into_iter().map(SprotoValue::Str).collect())
    }
}

impl From<Vec<bool>> for SprotoValue {
    fn from(v: Vec<bool>) -> Self {
        SprotoValue::Array(v.into_iter().map(SprotoValue::Boolean).collect())
    }
}

// TryFrom implementations for extracting values from SprotoValue
impl TryFrom<SprotoValue> for i64 {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Integer(i) => Ok(i),
            _ => Err("expected integer"),
        }
    }
}

impl TryFrom<SprotoValue> for String {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Str(s) => Ok(s),
            _ => Err("expected string"),
        }
    }
}

impl TryFrom<SprotoValue> for bool {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Boolean(b) => Ok(b),
            SprotoValue::Integer(i) => Ok(i != 0),
            _ => Err("expected boolean"),
        }
    }
}

impl TryFrom<SprotoValue> for f64 {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Double(d) => Ok(d),
            SprotoValue::Integer(i) => Ok(i as f64),
            _ => Err("expected double"),
        }
    }
}

impl TryFrom<SprotoValue> for Vec<u8> {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Binary(b) => Ok(b),
            _ => Err("expected binary"),
        }
    }
}

impl TryFrom<SprotoValue> for Vec<i64> {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Array(arr) => {
                arr.into_iter()
                    .map(|item| i64::try_from(item))
                    .collect()
            }
            _ => Err("expected array"),
        }
    }
}

impl TryFrom<SprotoValue> for Vec<f64> {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Array(arr) => {
                arr.into_iter()
                    .map(|item| f64::try_from(item))
                    .collect()
            }
            _ => Err("expected array"),
        }
    }
}

impl TryFrom<SprotoValue> for Vec<String> {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Array(arr) => {
                arr.into_iter()
                    .map(|item| String::try_from(item))
                    .collect()
            }
            _ => Err("expected array"),
        }
    }
}

impl TryFrom<SprotoValue> for Vec<bool> {
    type Error = &'static str;
    fn try_from(v: SprotoValue) -> Result<Self, Self::Error> {
        match v {
            SprotoValue::Array(arr) => {
                arr.into_iter()
                    .map(|item| bool::try_from(item))
                    .collect()
            }
            _ => Err("expected array"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_conversions() {
        assert_eq!(SprotoValue::from(42i64), SprotoValue::Integer(42));
        assert_eq!(SprotoValue::from(true), SprotoValue::Boolean(true));
        assert_eq!(SprotoValue::from("hello"), SprotoValue::Str("hello".into()));
        assert_eq!(SprotoValue::from(3.14f64), SprotoValue::Double(3.14));
    }

    #[test]
    fn test_struct_builder() {
        let val = SprotoValue::from_fields(vec![
            ("name", "Alice".into()),
            ("age", 13i64.into()),
        ]);
        assert_eq!(val.get("name"), Some(&SprotoValue::Str("Alice".into())));
        assert_eq!(val.get("age"), Some(&SprotoValue::Integer(13)));
        assert_eq!(val.get("missing"), None);
    }

    #[test]
    fn test_equality() {
        let a = SprotoValue::Double(0.1);
        let b = SprotoValue::Double(0.1);
        assert_eq!(a, b);

        let a = SprotoValue::Integer(1);
        let b = SprotoValue::Boolean(true);
        assert_ne!(a, b);
    }
}
