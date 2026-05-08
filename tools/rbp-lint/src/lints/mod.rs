use std::path::Path;

use ra_ap_syntax::SourceFile;

use crate::diagnostic::Diagnostic;

pub mod arc_clone;
pub mod bool_option_pair;
pub mod dead_code_comment;
pub mod debug_print;
pub mod hardcoded_secret;
pub mod lazy_static_macro;
pub mod manual_let_else;
pub mod mod_rs_file;
pub mod needless_return;
pub mod no_expect;
pub mod no_panic;
pub mod no_unwrap;
pub mod non_exhaustive_pub_error;
pub mod pub_field_newtype;
pub mod raw_id_field;
pub mod status_string_field;
pub mod string_as_error;
pub mod tracing_format;
pub mod unbounded_channel;
pub mod unsafe_safety_comment;
pub mod unwrap_or_default_call;
pub mod util;

pub struct LintContext<'a> {
    pub file: &'a Path,
    pub source: &'a str,
    pub tree: &'a SourceFile,
}

pub trait LintRule: Send + Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>);
}

pub fn all_lints() -> Vec<Box<dyn LintRule>> {
    vec![
        Box::new(no_unwrap::NoUnwrap),
        Box::new(no_expect::NoExpect),
        Box::new(no_panic::NoPanic),
        Box::new(dead_code_comment::DeadCodeComment),
        Box::new(tracing_format::TracingFormat),
        Box::new(arc_clone::ArcCloneExplicit),
        Box::new(hardcoded_secret::HardcodedSecret),
        Box::new(unsafe_safety_comment::UnsafeSafetyComment),
        Box::new(debug_print::DebugPrint),
        Box::new(string_as_error::StringAsError),
        Box::new(unbounded_channel::UnboundedChannel),
        Box::new(unwrap_or_default_call::UnwrapOrDefaultCall),
        Box::new(mod_rs_file::ModRsFile),
        Box::new(needless_return::NeedlessReturn),
        Box::new(lazy_static_macro::LazyStaticMacro),
        Box::new(manual_let_else::ManualLetElse),
        Box::new(pub_field_newtype::PubFieldNewtype),
        Box::new(non_exhaustive_pub_error::NonExhaustivePubError),
        Box::new(raw_id_field::RawIdField),
        Box::new(status_string_field::StatusStringField),
        Box::new(bool_option_pair::BoolOptionPair),
    ]
}
