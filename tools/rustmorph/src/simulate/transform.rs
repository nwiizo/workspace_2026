use crate::types::OwnershipKind;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A signature transformation to simulate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Transform {
    /// `&T` → `T` — borrow to owned.
    RefToOwned,
    /// `T` → `&T` — owned to borrow.
    OwnedToRef,
    /// `&T` → `&mut T` — shared to mutable borrow.
    RefToMutRef,
    /// `&mut T` → `&T` — mutable to shared borrow.
    MutRefToRef,
    /// `String` → `&str`.
    StringToStr,
    /// `&str` → `String`.
    StrToString,
    /// `Vec<T>` → `&[T]`.
    VecToSlice,
    /// `&[T]` → `Vec<T>`.
    SliceToVec,
    /// `Box<T>` → `T`.
    BoxToInline,
}

impl Transform {
    /// What ownership kind does the source parameter need to have?
    pub fn source_ownership(&self) -> OwnershipKind {
        match self {
            Self::RefToOwned | Self::RefToMutRef => OwnershipKind::Ref,
            Self::MutRefToRef => OwnershipKind::MutRef,
            Self::OwnedToRef
            | Self::StringToStr
            | Self::StrToString
            | Self::VecToSlice
            | Self::SliceToVec
            | Self::BoxToInline => OwnershipKind::Owned,
        }
    }

    /// What ownership kind does the parameter become after transform?
    pub fn target_ownership(&self) -> OwnershipKind {
        match self {
            Self::RefToOwned | Self::StrToString | Self::SliceToVec => OwnershipKind::Owned,
            Self::OwnedToRef | Self::MutRefToRef | Self::StringToStr | Self::VecToSlice => {
                OwnershipKind::Ref
            }
            Self::RefToMutRef => OwnershipKind::MutRef,
            Self::BoxToInline => OwnershipKind::Owned,
        }
    }

    /// Parse from a CLI string.
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "ref-to-owned" => Some(Self::RefToOwned),
            "owned-to-ref" => Some(Self::OwnedToRef),
            "ref-to-mut-ref" => Some(Self::RefToMutRef),
            "mut-ref-to-ref" => Some(Self::MutRefToRef),
            "string-to-str" => Some(Self::StringToStr),
            "str-to-string" => Some(Self::StrToString),
            "vec-to-slice" => Some(Self::VecToSlice),
            "slice-to-vec" => Some(Self::SliceToVec),
            "box-to-inline" => Some(Self::BoxToInline),
            _ => None,
        }
    }

    /// All transform variants.
    pub fn all() -> &'static [Transform] {
        &[
            Self::RefToOwned,
            Self::OwnedToRef,
            Self::RefToMutRef,
            Self::MutRefToRef,
            Self::StringToStr,
            Self::StrToString,
            Self::VecToSlice,
            Self::SliceToVec,
            Self::BoxToInline,
        ]
    }

    /// CLI name for this transform (inverse of `from_str_name`).
    pub fn cli_name(&self) -> &'static str {
        match self {
            Self::RefToOwned => "ref-to-owned",
            Self::OwnedToRef => "owned-to-ref",
            Self::RefToMutRef => "ref-to-mut-ref",
            Self::MutRefToRef => "mut-ref-to-ref",
            Self::StringToStr => "string-to-str",
            Self::StrToString => "str-to-string",
            Self::VecToSlice => "vec-to-slice",
            Self::SliceToVec => "slice-to-vec",
            Self::BoxToInline => "box-to-inline",
        }
    }

    /// All available transforms for help text.
    pub fn all_names() -> &'static [&'static str] {
        &[
            "ref-to-owned",
            "owned-to-ref",
            "ref-to-mut-ref",
            "mut-ref-to-ref",
            "string-to-str",
            "str-to-string",
            "vec-to-slice",
            "slice-to-vec",
            "box-to-inline",
        ]
    }
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RefToOwned => write!(f, "&T → T"),
            Self::OwnedToRef => write!(f, "T → &T"),
            Self::RefToMutRef => write!(f, "&T → &mut T"),
            Self::MutRefToRef => write!(f, "&mut T → &T"),
            Self::StringToStr => write!(f, "String → &str"),
            Self::StrToString => write!(f, "&str → String"),
            Self::VecToSlice => write!(f, "Vec<T> → &[T]"),
            Self::SliceToVec => write!(f, "&[T] → Vec<T>"),
            Self::BoxToInline => write!(f, "Box<T> → T"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_names() {
        for name in Transform::all_names() {
            assert!(
                Transform::from_str_name(name).is_some(),
                "failed to parse: {name}"
            );
        }
    }

    #[test]
    fn roundtrip_ownership() {
        let t = Transform::RefToOwned;
        assert_eq!(t.source_ownership(), OwnershipKind::Ref);
        assert_eq!(t.target_ownership(), OwnershipKind::Owned);
    }
}
