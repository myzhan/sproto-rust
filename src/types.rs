use std::collections::HashMap;

/// The type of a field in a sproto schema.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Integer,
    Boolean,
    String,
    Binary,
    Double,
    /// A user-defined struct type. The value is the index into `Sproto.types_list`.
    Struct(usize),
}

/// A field definition within a sproto type.
#[derive(Debug, Clone)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Field tag (unique within the type, ascending order).
    pub tag: u16,
    /// The base type of this field.
    pub field_type: FieldType,
    /// Whether this field is an array (prefixed with `*`).
    pub is_array: bool,
    /// For map arrays: the tag of the key field in the subtype.
    /// -1 if not a map.
    pub key_tag: i32,
    /// For `*Type()` two-field maps: interpret as map on decode.
    pub is_map: bool,
    /// For `integer(N)` fixed-point: the decimal precision (e.g., 100 for integer(2)).
    /// 0 if not a decimal field.
    pub decimal_precision: u32,
}

/// A user-defined type (struct/message) in the sproto schema.
#[derive(Debug, Clone)]
pub struct SprotoType {
    /// Type name (may include dots for nested types, e.g. "Person.PhoneNumber").
    pub name: String,
    /// Fields sorted by tag in ascending order.
    pub fields: Vec<Field>,
    /// If tags are contiguous starting from base_tag, this is the base tag.
    /// -1 means tags are not contiguous (use binary search for lookup).
    pub base_tag: i32,
    /// Maximum number of field slots including skip entries.
    /// This is used to pre-allocate header space during encoding.
    pub maxn: usize,
}

/// A protocol definition for RPC.
#[derive(Debug, Clone)]
pub struct Protocol {
    /// Protocol name.
    pub name: String,
    /// Protocol tag number.
    pub tag: u16,
    /// Index into `Sproto.types_list` for the request type, if any.
    pub request: Option<usize>,
    /// Index into `Sproto.types_list` for the response type, if any.
    pub response: Option<usize>,
    /// If true, response is explicitly nil (no response expected).
    pub confirm: bool,
}

/// The top-level sproto schema container, holding all types and protocols.
#[derive(Debug, Clone)]
pub struct Sproto {
    /// All types in definition order.
    pub types_list: Vec<SprotoType>,
    /// Map from type name to index in `types_list`.
    pub types_by_name: HashMap<String, usize>,
    /// All protocols sorted by tag.
    pub protocols: Vec<Protocol>,
    /// Map from protocol name to index in `protocols`.
    pub protocols_by_name: HashMap<String, usize>,
    /// Map from protocol tag to index in `protocols`.
    pub protocols_by_tag: HashMap<u16, usize>,
}

impl SprotoType {
    /// Find a field by tag, using direct indexing if tags are contiguous,
    /// otherwise binary search. Mirrors the C `findtag()` function.
    pub fn find_field_by_tag(&self, tag: u16) -> Option<&Field> {
        if self.base_tag >= 0 {
            let idx = tag as i32 - self.base_tag;
            if idx < 0 || idx as usize >= self.fields.len() {
                return None;
            }
            Some(&self.fields[idx as usize])
        } else {
            // Binary search
            self.fields
                .binary_search_by_key(&tag, |f| f.tag)
                .ok()
                .map(|idx| &self.fields[idx])
        }
    }

    /// Find a field by name.
    pub fn find_field_by_name(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl Sproto {
    /// Create an empty Sproto.
    pub fn new() -> Self {
        Sproto {
            types_list: Vec::new(),
            types_by_name: HashMap::new(),
            protocols: Vec::new(),
            protocols_by_name: HashMap::new(),
            protocols_by_tag: HashMap::new(),
        }
    }

    /// Get a type by name.
    pub fn get_type(&self, name: &str) -> Option<&SprotoType> {
        self.types_by_name
            .get(name)
            .map(|&idx| &self.types_list[idx])
    }

    /// Get a type index by name.
    pub fn get_type_index(&self, name: &str) -> Option<usize> {
        self.types_by_name.get(name).copied()
    }

    /// Get a protocol by name.
    pub fn get_protocol(&self, name: &str) -> Option<&Protocol> {
        self.protocols_by_name
            .get(name)
            .map(|&idx| &self.protocols[idx])
    }

    /// Get a protocol by tag.
    pub fn get_protocol_by_tag(&self, tag: u16) -> Option<&Protocol> {
        self.protocols_by_tag
            .get(&tag)
            .map(|&idx| &self.protocols[idx])
    }
}

impl Default for Sproto {
    fn default() -> Self {
        Self::new()
    }
}
