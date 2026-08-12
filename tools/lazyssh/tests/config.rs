use std::fs;

use lazyssh::config::load;
use tempfile::tempdir;

#[test]
fn loads_concrete_hosts_and_their_connection_fields() {
    let dir = tempdir().expect("temp dir");
    let config = dir.path().join("config");
    fs::write(
        &config,
        r#"
Host *
    ServerAliveInterval 30

Host prod-web prod-api
    HostName 10.0.0.8
    User deploy
    Port 2222
    IdentityFile "~/.ssh/prod key"
    ProxyJump bastion

Host *.internal !legacy.internal
    User ignored

Host -oProxyCommand=unsafe
    HostName ignored.example.com
"#,
    )
    .expect("write config");

    let hosts = load(&config).expect("load config");

    assert_eq!(hosts.len(), 2);
    assert_eq!(hosts[0].alias, "prod-web");
    assert_eq!(hosts[1].alias, "prod-api");
    for host in hosts {
        assert_eq!(host.host_name.as_deref(), Some("10.0.0.8"));
        assert_eq!(host.user.as_deref(), Some("deploy"));
        assert_eq!(host.port, Some(2222));
        assert_eq!(host.identity_files, ["~/.ssh/prod key"]);
        assert_eq!(host.proxy_jump.as_deref(), Some("bastion"));
        assert_eq!(host.line, 5);
    }
}

#[test]
fn expands_includes_and_ignores_include_cycles() {
    let dir = tempdir().expect("temp dir");
    let parts = dir.path().join("conf.d");
    fs::create_dir(&parts).expect("create include dir");
    let config = dir.path().join("config");
    let included = parts.join("work.conf");

    fs::write(
        &config,
        format!(
            "Include {}\nHost personal\n  User me\n",
            parts.join("*.conf").display()
        ),
    )
    .expect("write root config");
    fs::write(
        &included,
        format!(
            "Include {}\nHost work\n  HostName work.example.com\n",
            config.display()
        ),
    )
    .expect("write included config");

    let hosts = load(&config).expect("load config");

    assert_eq!(
        hosts
            .iter()
            .map(|host| host.alias.as_str())
            .collect::<Vec<_>>(),
        ["work", "personal"]
    );
    assert_eq!(
        hosts[0].source,
        included.canonicalize().expect("canonical include path")
    );
}

#[test]
fn keeps_the_first_value_when_an_alias_is_declared_more_than_once() {
    let dir = tempdir().expect("temp dir");
    let config = dir.path().join("config");
    fs::write(
        &config,
        "Host db\n  HostName primary.example.com\nHost db\n  HostName fallback.example.com\n  User ops\n",
    )
    .expect("write config");

    let hosts = load(&config).expect("load config");

    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].host_name.as_deref(), Some("primary.example.com"));
    assert_eq!(hosts[0].user.as_deref(), Some("ops"));
}

#[test]
fn does_not_present_conditional_match_options_as_direct_host_values() {
    let dir = tempdir().expect("temp dir");
    let config = dir.path().join("config");
    fs::write(
        &config,
        "Host prod\n  HostName prod.example.com\nMatch host prod\n  User conditional\n",
    )
    .expect("write config");

    let hosts = load(&config).expect("load config");

    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].user, None);
}
