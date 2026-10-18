//! Error codes: the classes of failure a caller branches on.

use std::any::TypeId;
use std::fmt;
use std::hash::{Hash, Hasher};

/// A type whose values are error codes.
///
/// A contract declares the codes its callers may branch on as a protobuf
/// `enum` next to its `service`. Every enum that `protoc-gen-buffa`
/// generates is an `ErrorCode` already, through `buffa::Enumeration`. A
/// Rust enum implements the three methods by hand.
///
/// ```
/// use protocontract::{Error, ErrorCode};
///
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum StoreCode {
///     Full = 1,
/// }
///
/// impl ErrorCode for StoreCode {
///     fn to_i32(self) -> i32 {
///         self as i32
///     }
///     fn from_i32(value: i32) -> Option<StoreCode> {
///         (value == 1).then_some(StoreCode::Full)
///     }
///     fn name(self) -> &'static str {
///         "STORE_FULL"
///     }
/// }
///
/// let error = Error::new(StoreCode::Full, "no room for the record");
/// assert_eq!(error.code_as::<StoreCode>(), Some(StoreCode::Full));
/// ```
pub trait ErrorCode: Copy + Send + Sync + 'static {
    /// The number of this code, unique within its type.
    fn to_i32(self) -> i32;

    /// The code of this type with the number `value`, or `None` when the
    /// type has no such code.
    fn from_i32(value: i32) -> Option<Self>;

    /// The name of this code, as a developer reads it in a message.
    fn name(self) -> &'static str;
}

impl<T: buffa::Enumeration + Send + Sync + 'static> ErrorCode for T {
    fn to_i32(self) -> i32 {
        buffa::Enumeration::to_i32(&self)
    }

    fn from_i32(value: i32) -> Option<T> {
        <T as buffa::Enumeration>::from_i32(value)
    }

    fn name(self) -> &'static str {
        buffa::Enumeration::proto_name(&self)
    }
}

/// An error code of any [`ErrorCode`] type, with that type erased.
///
/// Two codes are equal when they are the same value of the same type: code
/// 1 of one enum never equals code 1 of another. [`Code::get`] gives the
/// typed code back.
///
/// ```
/// use protocontract::{Code, RuntimeCode};
///
/// let code = Code::of(RuntimeCode::Panicked);
/// assert!(code.is(RuntimeCode::Panicked));
/// assert_eq!(code.get::<RuntimeCode>(), Some(RuntimeCode::Panicked));
/// assert_eq!(code.to_string(), "PANICKED");
/// ```
#[derive(Clone, Copy)]
pub struct Code {
    kind: TypeId,
    value: i32,
    name: &'static str,
}

impl Code {
    /// The erased form of `code`.
    #[must_use]
    pub fn of<C: ErrorCode>(code: C) -> Code {
        Code {
            kind: TypeId::of::<C>(),
            value: code.to_i32(),
            name: code.name(),
        }
    }

    /// This code as a `C`, or `None` when it is a code of another type.
    #[must_use]
    pub fn get<C: ErrorCode>(self) -> Option<C> {
        if self.kind == TypeId::of::<C>() {
            C::from_i32(self.value)
        } else {
            None
        }
    }

    /// Whether this code is `code`.
    #[must_use]
    pub fn is<C: ErrorCode>(self, code: C) -> bool {
        self == Code::of(code)
    }

    /// The number of this code within its type.
    #[must_use]
    pub fn value(self) -> i32 {
        self.value
    }

    /// The name of this code.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.name
    }
}

impl<C: ErrorCode> From<C> for Code {
    fn from(code: C) -> Code {
        Code::of(code)
    }
}

impl PartialEq for Code {
    fn eq(&self, other: &Code) -> bool {
        self.kind == other.kind && self.value == other.value
    }
}

impl Eq for Code {}

impl Hash for Code {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.value.hash(state);
    }
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Code({}={})", self.name, self.value)
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// The failures the runtime itself reports, outside any contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuntimeCode {
    /// The implementation panicked on another thread (`Offload`); the
    /// message carries the panic's message when it has one.
    Panicked = 1,
    /// A sync call through `Blocking` was made inside a current-thread tokio
    /// runtime, where waiting would stop the runtime the call needs.
    CannotBlock = 2,
    /// The runtime dropped the call before it finished, typically because
    /// it is shutting down.
    Cancelled = 3,
    /// A reference to a service holds no implementation: it was never
    /// given one, or it was decoded (a reference does not survive
    /// encoding).
    Unbound = 4,
    /// An async implementation was asked for in the blocking form where no
    /// tokio runtime could run it: outside a runtime, or in a build without
    /// the `tokio` feature.
    NoRuntime = 5,
}

impl ErrorCode for RuntimeCode {
    fn to_i32(self) -> i32 {
        self as i32
    }

    fn from_i32(value: i32) -> Option<RuntimeCode> {
        match value {
            1 => Some(RuntimeCode::Panicked),
            2 => Some(RuntimeCode::CannotBlock),
            3 => Some(RuntimeCode::Cancelled),
            4 => Some(RuntimeCode::Unbound),
            5 => Some(RuntimeCode::NoRuntime),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            RuntimeCode::Panicked => "PANICKED",
            RuntimeCode::CannotBlock => "CANNOT_BLOCK",
            RuntimeCode::Cancelled => "CANCELLED",
            RuntimeCode::Unbound => "UNBOUND",
            RuntimeCode::NoRuntime => "NO_RUNTIME",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{Code, ErrorCode, RuntimeCode};

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Other {
        One = 1,
    }

    impl ErrorCode for Other {
        fn to_i32(self) -> i32 {
            self as i32
        }
        fn from_i32(value: i32) -> Option<Other> {
            (value == 1).then_some(Other::One)
        }
        fn name(self) -> &'static str {
            "ONE"
        }
    }

    #[test]
    fn runtime_codes_round_trip() {
        for code in [
            RuntimeCode::Panicked,
            RuntimeCode::CannotBlock,
            RuntimeCode::Cancelled,
            RuntimeCode::Unbound,
            RuntimeCode::NoRuntime,
        ] {
            assert_eq!(RuntimeCode::from_i32(code.to_i32()), Some(code));
            assert_eq!(Code::of(code).get::<RuntimeCode>(), Some(code));
        }
        assert_eq!(RuntimeCode::from_i32(0), None);
    }

    #[test]
    fn same_number_of_another_type_is_another_code() {
        let runtime = Code::of(RuntimeCode::Panicked);
        let other = Code::of(Other::One);
        assert_eq!(runtime.value(), other.value());
        assert_ne!(runtime, other);
        assert_eq!(other.get::<RuntimeCode>(), None);
        assert!(!other.is(RuntimeCode::Panicked));
        let set: HashSet<Code> = [runtime, other, Code::of(Other::One)].into();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn display_and_debug_show_the_name() {
        let code = Code::from(RuntimeCode::CannotBlock);
        assert_eq!(code.to_string(), "CANNOT_BLOCK");
        assert_eq!(format!("{code:?}"), "Code(CANNOT_BLOCK=2)");
    }
}
