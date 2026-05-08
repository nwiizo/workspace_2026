use ra_ap_syntax::TextRange;

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct ModRsFile;

impl LintRule for ModRsFile {
    fn id(&self) -> &'static str {
        "mod-rs-file"
    }

    fn description(&self) -> &'static str {
        "Edition 2018+: prefer `foo.rs + foo/` over `foo/mod.rs`"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        let Some(name) = ctx.file.file_name().and_then(|s| s.to_str()) else {
            return;
        };
        if name != "mod.rs" {
            return;
        }
        // Anchor diagnostic to start-of-file.
        let range = TextRange::empty(0.into());
        out.push(Diagnostic::from_range(
            ctx.file,
            self.id(),
            Severity::Warning,
            "`mod.rs` files are discouraged in Edition 2018+",
            ctx.source,
            range,
            Some(
                "rename to `foo.rs` next to a `foo/` directory; \
                 e.g. move `foo/mod.rs` → `foo.rs` and keep submodules in `foo/*.rs`"
                    .into(),
            ),
        ));
    }
}
