/// This module holds the two dom implementations we use. We distingush between
/// owned and borrowed. The difference being is that the borrowed value will
/// use `&str` as its string type, refferencing the input, while owned will
/// allocate a new String for each value.
///
/// Note that since json strings allow for for escape sequences the borrowed
/// value does not impement zero copy parsing, it does however not allocate
/// new memory for strings.
///
/// This differs notably from serds zero copy implementation as, unlike serde,
/// we do not require prior knowledge sbout string comtent to to take advantage
/// of it.

/// Borrowed values, using Cow's for strings using in situ parsing strategies wherever possible
pub mod borrowed;
pub(crate) mod generator;
/// Owned, lifetimeless version of the value for times when lifetimes are to be avoided
pub mod owned;

pub use self::borrowed::{deserialize as deserialize_borrowed_value, Value as BorrowedValue};
pub use self::owned::{deserialize as deserialize_owned_value, Value as OwnedValue};
use crate::numberparse::Number;
use crate::{stry, unlikely, Deserializer, ErrorType, Result};
use halfbrown::HashMap;
use std::borrow::Borrow;
use std::convert::TryInto;
use std::hash::Hash;
use std::marker::PhantomData;

/// Types of JSON values
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ValueType {
    /// null
    Null,
    /// a boolean
    Bool,
    /// a signed integer type
    I64,
    /// a unsigned integer type
    U64,
    /// a float type
    F64,
    /// a string type
    String,
    /// an array
    Array,
    /// an object
    Object,
}

/// The `ValueTrait` exposes common interface for values, this allows using both
/// `BorrowedValue` and `OwnedValue` nearly interchangable
pub trait ValueTrait:
    From<i8>
    + From<i16>
    + From<i32>
    + From<i64>
    + From<u8>
    + From<u16>
    + From<u32>
    + From<u64>
    + From<f32>
    + From<f64>
    + From<String>
    + From<bool>
    + From<()>
    + PartialEq<i8>
    + PartialEq<i16>
    + PartialEq<i32>
    + PartialEq<i64>
    + PartialEq<i128>
    + PartialEq<u8>
    + PartialEq<u16>
    + PartialEq<u32>
    + PartialEq<u64>
    + PartialEq<u128>
    + PartialEq<f32>
    + PartialEq<f64>
    + PartialEq<String>
    + PartialEq<bool>
    + PartialEq<()>
{
    /// The type for Objects
    type Key;

    /// Returns an empty array
    fn array() -> Self;
    /// Returns an empty object
    fn object() -> Self;
    /// Returns anull value
    fn null() -> Self;

    /// Gets a ref to a value based on a key, returns `None` if the
    /// current Value isn't an Object or doesn't contain the key
    /// it was asked for.
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<&Self>
    where
        Self::Key: Borrow<Q> + Hash + Eq,
        Q: Hash + Eq,
    {
        self.as_object().and_then(|a| a.get(k))
    }

    /// Same as `get` but returns a mutable ref instead
    //    fn get_amut(&mut self, k: &str) -> Option<&mut Self>;
    fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut Self>
    where
        Self::Key: Borrow<Q> + Hash + Eq,
        Q: Hash + Eq,
    {
        self.as_object_mut().and_then(|m| m.get_mut(&k))
    }

    /// Gets a ref to a value based on n index, returns `None` if the
    /// current Value isn't an Array or doesn't contain the index
    /// it was asked for.
    fn get_idx(&self, i: usize) -> Option<&Self> {
        self.as_array().and_then(|a| a.get(i))
    }

    /// Same as `get_idx` but returns a mutable ref instead
    fn get_idx_mut(&mut self, i: usize) -> Option<&mut Self> {
        self.as_array_mut().and_then(|a| a.get_mut(i))
    }

    /// Returns the type of the current Valye
    fn value_type(&self) -> ValueType;

    /// returns true if the current value is null
    fn is_null(&self) -> bool;

    /// Tries to represent the value as a bool
    fn as_bool(&self) -> Option<bool>;
    /// returns true if the current value a bool
    fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Tries to represent the value as an i128
    fn as_i128(&self) -> Option<i128> {
        self.as_i64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a i128
    fn is_i128(&self) -> bool {
        self.as_i128().is_some()
    }

    /// Tries to represent the value as an i64
    fn as_i64(&self) -> Option<i64>;
    /// returns true if the current value can be represented as a i64
    fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// Tries to represent the value as an i32
    fn as_i32(&self) -> Option<i32> {
        self.as_i64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a i32
    fn is_i32(&self) -> bool {
        self.as_i32().is_some()
    }

    /// Tries to represent the value as an i16
    fn as_i16(&self) -> Option<i16> {
        self.as_i64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a i16
    fn is_i16(&self) -> bool {
        self.as_i16().is_some()
    }

    /// Tries to represent the value as an i8
    fn as_i8(&self) -> Option<i8> {
        self.as_i64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a i8
    fn is_i8(&self) -> bool {
        self.as_i8().is_some()
    }

    /// Tries to represent the value as an u128
    fn as_u128(&self) -> Option<u128> {
        self.as_u64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a u128
    fn is_u128(&self) -> bool {
        self.as_u128().is_some()
    }

    /// Tries to represent the value as an u64
    fn as_u64(&self) -> Option<u64>;

    /// returns true if the current value can be represented as a u64
    fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    /// Tries to represent the value as an usize
    fn as_usize(&self) -> Option<usize> {
        self.as_u64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a usize
    fn is_usize(&self) -> bool {
        self.as_usize().is_some()
    }

    /// Tries to represent the value as an u32
    fn as_u32(&self) -> Option<u32> {
        self.as_u64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a u32
    fn is_u32(&self) -> bool {
        self.as_u32().is_some()
    }

    /// Tries to represent the value as an u16
    fn as_u16(&self) -> Option<u16> {
        self.as_u64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a u16
    fn is_u16(&self) -> bool {
        self.as_u16().is_some()
    }

    /// Tries to represent the value as an u8
    fn as_u8(&self) -> Option<u8> {
        self.as_u64().and_then(|u| u.try_into().ok())
    }
    /// returns true if the current value can be represented as a u8
    fn is_u8(&self) -> bool {
        self.as_u8().is_some()
    }

    /// Tries to represent the value as a f64
    fn as_f64(&self) -> Option<f64>;
    /// returns true if the current value can be represented as a f64
    fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }
    /// Casts the current value to a f64 if possible, this will turn integer
    /// values into floats.
    fn cast_f64(&self) -> Option<f64>;
    /// returns true if the current value can be cast into a f64
    fn is_f64_castable(&self) -> bool {
        self.cast_f64().is_some()
    }

    /// Tries to represent the value as a f32
    #[allow(clippy::cast_possible_truncation)]
    fn as_f32(&self) -> Option<f32> {
        self.as_f64().and_then(|u| {
            if u <= f64::from(std::f32::MAX) && u >= f64::from(std::f32::MIN) {
                // Since we check above
                Some(u as f32)
            } else {
                None
            }
        })
    }
    /// returns true if the current value can be represented as a f64
    fn is_f32(&self) -> bool {
        self.as_f32().is_some()
    }

    /// Tries to represent the value as a &str
    fn as_str(&self) -> Option<&str>;
    /// returns true if the current value can be represented as a str
    fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    /// Tries to represent the value as an array and returns a refference to it
    fn as_array(&self) -> Option<&Vec<Self>>;
    /// Tries to represent the value as an array and returns a mutable refference to it
    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>>;
    /// returns true if the current value can be represented as an array
    fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// Tries to represent the value as an object and returns a refference to it
    fn as_object(&self) -> Option<&HashMap<Self::Key, Self>>;
    /// Tries to represent the value as an object and returns a mutable refference to it
    fn as_object_mut(&mut self) -> Option<&mut HashMap<Self::Key, Self>>;
    /// returns true if the current value can be represented as an object
    fn is_object(&self) -> bool {
        self.as_object().is_some()
    }
}

/// Deserializes towards an `ValueTrait` implementation
pub fn deserialize<'de, V>(s: &'de mut [u8]) -> Result<V>
where
    V: ValueTrait
        + 'de
        + From<&'de str>
        + From<Number>
        + From<HashMap<<V as ValueTrait>::Key, V>>
        + From<Vec<V>>,
    <V as ValueTrait>::Key: Eq + Hash + From<&'de str>,
{
    let de = stry!(Deserializer::from_slice(s));
    ValueDeserializer::from_deserializer(de).parse()
}

struct ValueDeserializer<'de, V>
where
    V: ValueTrait,
{
    de: Deserializer<'de>,
    marker: PhantomData<V>,
}

impl<'de, V> ValueDeserializer<'de, V>
where
    V: ValueTrait
        + 'de
        + From<&'de str>
        + From<Number>
        + From<HashMap<<V as ValueTrait>::Key, V>>
        + From<Vec<V>>,
    <V as ValueTrait>::Key: Eq + Hash + From<&'de str>,
{
    pub fn from_deserializer(de: Deserializer<'de>) -> Self {
        Self {
            de,
            marker: PhantomData::default(),
        }
    }

    #[cfg_attr(not(feature = "no-inline"), inline(always))]
    pub fn parse(&mut self) -> Result<V> {
        match self.de.next_() {
            b'"' => self.de.parse_str_().map(V::from),
            b'-' => self.de.parse_number_root(true).map(V::from),
            b'0'..=b'9' => self.de.parse_number_root(false).map(V::from),
            b'n' => Ok(V::null()),
            b't' => Ok(V::from(true)),
            b'f' => Ok(V::from(false)),
            b'[' => self.parse_array(),
            b'{' => self.parse_map(),
            _c => Err(self.de.error(ErrorType::UnexpectedCharacter)),
        }
    }

    #[cfg_attr(not(feature = "no-inline"), inline(always))]
    fn parse_value(&mut self) -> Result<V> {
        match self.de.next_() {
            b'"' => self.de.parse_str_().map(V::from),
            b'-' => self.de.parse_number_(true).map(V::from),
            b'0'..=b'9' => self.de.parse_number_(false).map(V::from),
            b'n' => Ok(V::from(())),
            b't' => Ok(V::from(true)),
            b'f' => Ok(V::from(false)),
            b'[' => self.parse_array(),
            b'{' => self.parse_map(),
            _c => Err(self.de.error(ErrorType::UnexpectedCharacter)),
        }
    }

    #[cfg_attr(not(feature = "no-inline"), inline(always))]
    fn parse_array(&mut self) -> Result<V> {
        let es = self.de.count_elements();
        if unlikely!(es == 0) {
            self.de.skip();
            return Ok(V::array());
        }
        let mut res: Vec<V> = Vec::with_capacity(es);

        for _i in 0..es {
            res.push(stry!(self.parse_value()));
            self.de.skip();
        }
        Ok(V::from(res))
    }

    #[cfg_attr(not(feature = "no-inline"), inline(always))]
    fn parse_map(&mut self) -> Result<V> {
        // We short cut for empty arrays
        let es = self.de.count_elements();

        if unlikely!(es == 0) {
            self.de.skip();
            return Ok(V::object());
        }

        let mut res: HashMap<V::Key, V> = HashMap::with_capacity(es);

        // Since we checked if it's empty we know that we at least have one
        // element so we eat this

        for _ in 0..es {
            self.de.skip();
            let key = stry!(self.de.parse_str_());
            // We have to call parse short str twice since parse_short_str
            // does not move the cursor forward
            self.de.skip();
            res.insert_nocheck(key.into(), stry!(self.parse_value()));
            self.de.skip();
        }
        Ok(V::from(res))
    }
}
