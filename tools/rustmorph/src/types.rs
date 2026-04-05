use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// How a value is owned or borrowed at a given point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnershipKind {
    /// Owned value (`T`, `String`, `Vec<T>`, `Box<T>`)
    Owned,
    /// Shared reference (`&T`, `&str`, `&[T]`)
    Ref,
    /// Mutable reference (`&mut T`)
    MutRef,
}

impl fmt::Display for OwnershipKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owned => write!(f, "owned"),
            Self::Ref => write!(f, "&"),
            Self::MutRef => write!(f, "&mut"),
        }
    }
}

/// Detailed type information extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeInfo {
    /// The ownership kind at the surface level.
    pub ownership: OwnershipKind,
    /// The raw type name as written in source (e.g. `String`, `&str`, `Vec<Config>`).
    pub raw: String,
    /// The inner type name without reference/wrapper (e.g. `str`, `Config`, `[u8]`).
    pub inner: String,
    /// Lifetime annotation if present (e.g. `'a`).
    pub lifetime: Option<String>,
    /// Whether this type is generic (contains type parameters).
    pub is_generic: bool,
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// Source location information.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanInfo {
    pub file: PathBuf,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for SpanInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file.display(), self.line)
    }
}

/// A function or method parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamInfo {
    /// Parameter name (e.g. `data`, `self`).
    pub name: String,
    /// Type information.
    pub type_info: TypeInfo,
    /// Source location.
    pub span: SpanInfo,
}

/// A function/method signature extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Fully qualified name (e.g. `module::submodule::func_name`).
    pub name: String,
    /// Simple name without module path.
    pub short_name: String,
    /// If this is a method, the impl target type.
    pub impl_target: Option<String>,
    /// Parameters (including self if method).
    pub params: Vec<ParamInfo>,
    /// Return type, if any.
    pub return_type: Option<TypeInfo>,
    /// Source location of the function definition.
    pub span: SpanInfo,
}

impl fmt::Display for FunctionSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.short_name)?;
        for (i, p) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", p.name, p.type_info)?;
        }
        write!(f, ")")?;
        if let Some(ref ret) = self.return_type {
            write!(f, " -> {ret}")?;
        }
        Ok(())
    }
}

/// How a parameter is used at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArgUsage {
    /// Passed by move.
    Move,
    /// Passed by shared reference.
    Borrow,
    /// Passed by mutable reference.
    BorrowMut,
    /// `.clone()` is called before passing.
    Clone,
}

impl fmt::Display for ArgUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Move => write!(f, "move"),
            Self::Borrow => write!(f, "borrow"),
            Self::BorrowMut => write!(f, "borrow_mut"),
            Self::Clone => write!(f, "clone"),
        }
    }
}

/// An argument at a call site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallArg {
    /// The expression text.
    pub expr: String,
    /// How the argument is used.
    pub usage: ArgUsage,
}

/// A function call site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    /// Name of the calling function.
    pub caller: String,
    /// Name of the called function.
    pub callee: String,
    /// Arguments with usage info.
    pub args: Vec<CallArg>,
    /// Source location of the call.
    pub span: SpanInfo,
}
