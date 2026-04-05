use crate::types::{OwnershipKind, TypeInfo};
use syn::Type;

/// Analyze a `syn::Type` and extract ownership information.
pub fn analyze_type(ty: &Type) -> TypeInfo {
    let raw = quote::quote!(#ty).to_string();
    match ty {
        Type::Reference(r) => {
            let ownership = if r.mutability.is_some() {
                OwnershipKind::MutRef
            } else {
                OwnershipKind::Ref
            };
            let lifetime = r.lifetime.as_ref().map(|lt| lt.to_string());
            let elem = &r.elem;
            let inner = quote::quote!(#elem).to_string();
            let is_generic = contains_generic(&r.elem);
            TypeInfo {
                ownership,
                raw,
                inner,
                lifetime,
                is_generic,
            }
        }
        Type::Path(tp) => {
            let is_generic = !tp
                .path
                .segments
                .last()
                .is_none_or(|seg| matches!(seg.arguments, syn::PathArguments::None));
            let type_name = tp
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            TypeInfo {
                ownership: OwnershipKind::Owned,
                raw,
                inner: type_name,
                lifetime: None,
                is_generic,
            }
        }
        Type::Slice(_) => TypeInfo {
            ownership: OwnershipKind::Ref,
            raw,
            inner: quote::quote!(#ty).to_string(),
            lifetime: None,
            is_generic: false,
        },
        _ => TypeInfo {
            ownership: OwnershipKind::Owned,
            raw: raw.clone(),
            inner: raw,
            lifetime: None,
            is_generic: false,
        },
    }
}

fn contains_generic(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => tp
            .path
            .segments
            .iter()
            .any(|seg| !matches!(seg.arguments, syn::PathArguments::None)),
        Type::Reference(r) => contains_generic(&r.elem),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_type(s: &str) -> Type {
        syn::parse_str(s).expect("failed to parse type")
    }

    #[test]
    fn owned_string() {
        let info = analyze_type(&parse_type("String"));
        assert_eq!(info.ownership, OwnershipKind::Owned);
        assert_eq!(info.inner, "String");
        assert!(!info.is_generic);
    }

    #[test]
    fn shared_ref() {
        let info = analyze_type(&parse_type("&Config"));
        assert_eq!(info.ownership, OwnershipKind::Ref);
        assert!(info.lifetime.is_none());
    }

    #[test]
    fn mut_ref() {
        let info = analyze_type(&parse_type("&mut Vec<u8>"));
        assert_eq!(info.ownership, OwnershipKind::MutRef);
    }

    #[test]
    fn ref_with_lifetime() {
        let info = analyze_type(&parse_type("&'a str"));
        assert_eq!(info.ownership, OwnershipKind::Ref);
        assert_eq!(info.lifetime.as_deref(), Some("'a"));
    }

    #[test]
    fn generic_vec() {
        let info = analyze_type(&parse_type("Vec<Config>"));
        assert_eq!(info.ownership, OwnershipKind::Owned);
        assert!(info.is_generic);
    }
}
