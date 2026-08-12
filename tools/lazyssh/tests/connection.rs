use std::ffi::OsStr;
use std::path::Path;

use lazyssh::connection::ssh_command;

#[test]
fn passes_the_alias_to_openssh_as_one_literal_argument() {
    let command = ssh_command(Path::new("fixtures/config"), "prod;echo unsafe");

    assert_eq!(command.get_program(), OsStr::new("ssh"));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        [
            OsStr::new("-F"),
            OsStr::new("fixtures/config"),
            OsStr::new("prod;echo unsafe")
        ]
    );
}
