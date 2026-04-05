mod call_visitor;
mod ownership;
pub(crate) mod parser;
mod project;

pub use ownership::analyze_type;
pub use project::ProjectAnalysis;
