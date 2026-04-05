use crate::graph::DepNode;
use crate::scan::config::{ScanConfig, ScanJob};
use crate::simulate::Transform;
use crate::types::ParamInfo;

/// Should this function be skipped?
pub(crate) fn should_skip_function(node: &DepNode, config: &ScanConfig) -> bool {
    let name = &node.signature.name;

    if config.skip_test_functions && is_test_function(name) {
        return true;
    }
    if let Some(ref filter) = config.function_filter {
        if !name.contains(filter.as_str()) {
            return true;
        }
    }
    // Skip trait-mandated methods whose signatures cannot be changed.
    if is_trait_impl_method(&node.signature.short_name) {
        return true;
    }
    false
}

/// Should this parameter be skipped?
pub(crate) fn should_skip_param(param: &ParamInfo, config: &ScanConfig) -> bool {
    config.skip_self_params && param.name == "self"
}

/// Is this transform type-compatible with the parameter?
pub(crate) fn is_transform_type_compatible(param: &ParamInfo, transform: &Transform) -> bool {
    match transform {
        Transform::StringToStr => {
            param.type_info.inner == "String" || param.type_info.raw == "String"
        }
        Transform::StrToString => {
            param.type_info.inner == "str"
                || param.type_info.raw.contains("& str")
                || param.type_info.raw.contains("&str")
        }
        Transform::VecToSlice => {
            param.type_info.inner == "Vec"
                || param.type_info.raw.starts_with("Vec <")
                || param.type_info.raw.starts_with("Vec<")
        }
        Transform::SliceToVec => param.type_info.raw.contains('['),
        Transform::BoxToInline => {
            param.type_info.inner == "Box"
                || param.type_info.raw.starts_with("Box <")
                || param.type_info.raw.starts_with("Box<")
        }
        // Generic transforms apply to any type with matching ownership.
        _ => true,
    }
}

/// Is this a useful transform direction? Filters out cost-increasing or
/// rarely-desired transforms.
pub(crate) fn is_useful_transform(transform: &Transform, job: ScanJob) -> bool {
    match job {
        ScanJob::Full => {
            !matches!(
                transform,
                // &T → &mut T is almost never a desired refactoring direction.
                Transform::RefToMutRef
                // &str → String, &[T] → Vec<T> increase allocation cost.
                | Transform::StrToString
                | Transform::SliceToVec
            )
        }
        // Job-specific: already curated in transforms_for_job.
        ScanJob::CloneAudit | ScanJob::ApiSlim => true,
    }
}

/// Get the transforms applicable for a given job.
pub(crate) fn transforms_for_job(job: ScanJob) -> &'static [Transform] {
    match job {
        ScanJob::Full => Transform::all(),
        ScanJob::CloneAudit => &[Transform::RefToOwned, Transform::OwnedToRef],
        ScanJob::ApiSlim => &[
            Transform::StringToStr,
            Transform::VecToSlice,
            Transform::BoxToInline,
        ],
    }
}

/// Well-known trait methods whose signatures are dictated by the trait and
/// cannot be freely changed.
fn is_trait_impl_method(short_name: &str) -> bool {
    matches!(
        short_name,
        "fmt"
            | "from"
            | "into"
            | "try_from"
            | "try_into"
            | "default"
            | "clone"
            | "eq"
            | "ne"
            | "partial_cmp"
            | "cmp"
            | "hash"
            | "drop"
            | "deref"
            | "deref_mut"
            | "as_ref"
            | "as_mut"
            | "borrow"
            | "borrow_mut"
            | "index"
            | "index_mut"
            | "next"
            | "size_hint"
    )
}

fn is_test_function(name: &str) -> bool {
    name.split("::")
        .any(|seg| seg.starts_with("test_") || seg == "test" || seg == "tests")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OwnershipKind, SpanInfo, TypeInfo};
    use std::path::PathBuf;

    fn make_param(name: &str, raw: &str, inner: &str, ownership: OwnershipKind) -> ParamInfo {
        ParamInfo {
            name: name.to_string(),
            type_info: TypeInfo {
                ownership,
                raw: raw.to_string(),
                inner: inner.to_string(),
                lifetime: None,
                is_generic: false,
            },
            span: SpanInfo {
                file: PathBuf::from("test.rs"),
                line: 1,
                col: 0,
            },
        }
    }

    #[test]
    fn string_to_str_type_check() {
        let string_param = make_param("s", "String", "String", OwnershipKind::Owned);
        let vec_param = make_param("v", "Vec<u8>", "Vec", OwnershipKind::Owned);

        assert!(is_transform_type_compatible(
            &string_param,
            &Transform::StringToStr
        ));
        assert!(!is_transform_type_compatible(
            &vec_param,
            &Transform::StringToStr
        ));
    }

    #[test]
    fn vec_to_slice_type_check() {
        let vec_param = make_param("v", "Vec<u8>", "Vec", OwnershipKind::Owned);
        let string_param = make_param("s", "String", "String", OwnershipKind::Owned);

        assert!(is_transform_type_compatible(
            &vec_param,
            &Transform::VecToSlice
        ));
        assert!(!is_transform_type_compatible(
            &string_param,
            &Transform::VecToSlice
        ));
    }

    #[test]
    fn generic_transform_always_compatible() {
        let param = make_param("x", "Config", "Config", OwnershipKind::Owned);
        assert!(is_transform_type_compatible(&param, &Transform::OwnedToRef));
        assert!(is_transform_type_compatible(&param, &Transform::RefToOwned));
    }

    #[test]
    fn test_function_detection() {
        assert!(is_test_function("module::test_parse"));
        assert!(is_test_function("module::tests::helper"));
        assert!(!is_test_function("module::process"));
        assert!(!is_test_function("module::testing_utils"));
    }

    #[test]
    fn trait_impl_methods_skipped() {
        assert!(is_trait_impl_method("fmt"));
        assert!(is_trait_impl_method("from"));
        assert!(is_trait_impl_method("default"));
        assert!(!is_trait_impl_method("process"));
        assert!(!is_trait_impl_method("handle_request"));
    }

    #[test]
    fn full_scan_filters_cost_increasing_transforms() {
        assert!(!is_useful_transform(&Transform::RefToMutRef, ScanJob::Full));
        assert!(!is_useful_transform(&Transform::StrToString, ScanJob::Full));
        assert!(!is_useful_transform(&Transform::SliceToVec, ScanJob::Full));
        assert!(is_useful_transform(&Transform::RefToOwned, ScanJob::Full));
        assert!(is_useful_transform(&Transform::OwnedToRef, ScanJob::Full));
        assert!(is_useful_transform(&Transform::StringToStr, ScanJob::Full));
    }
}
