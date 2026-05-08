use ra_ap_syntax::ast::{self, AstNode};

use crate::diagnostic::{Diagnostic, Severity};
use crate::lints::{LintContext, LintRule};

pub struct UnboundedChannel;

impl LintRule for UnboundedChannel {
    fn id(&self) -> &'static str {
        "unbounded-channel"
    }

    fn description(&self) -> &'static str {
        "Unbounded channels can mask backpressure issues; prefer bounded channels"
    }

    fn check(&self, ctx: &LintContext<'_>, out: &mut Vec<Diagnostic>) {
        for node in ctx.tree.syntax().descendants() {
            let Some(call) = ast::CallExpr::cast(node) else {
                continue;
            };
            let Some(callee) = call.expr() else { continue };
            let text = callee.syntax().text().to_string();
            // Strip turbofish: `path::name::<T>` → `path::name`.
            let stripped = text.split("::<").next().unwrap_or(&text);
            let last = stripped.rsplit("::").next().unwrap_or("");
            if last != "unbounded_channel" {
                continue;
            }
            // Filter to channel-y paths to reduce false positives.
            let allowed = stripped.contains("mpsc")
                || stripped.contains("broadcast")
                || stripped.contains("channel")
                || stripped == "unbounded_channel";
            if !allowed {
                continue;
            }
            let range = call.syntax().text_range();
            out.push(Diagnostic::from_range(
                ctx.file,
                self.id(),
                Severity::Warning,
                "unbounded channel allocates without backpressure",
                ctx.source,
                range,
                Some(
                    "use `mpsc::channel(N)` with a bounded capacity unless the producer is \
                     guaranteed slower than the consumer"
                        .into(),
                ),
            ));
        }
    }
}
