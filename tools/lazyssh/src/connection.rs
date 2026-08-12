use std::path::Path;
use std::process::Command;

#[must_use]
pub fn ssh_command(config: &Path, alias: &str) -> Command {
    let mut command = Command::new("ssh");
    command.arg("-F").arg(config).arg(alias);
    command
}
