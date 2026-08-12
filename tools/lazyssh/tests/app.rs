use std::path::PathBuf;

use lazyssh::app::App;
use lazyssh::config::Host;

fn host(alias: &str, host_name: &str, user: &str) -> Host {
    Host {
        alias: alias.into(),
        host_name: Some(host_name.into()),
        user: Some(user.into()),
        port: None,
        identity_files: Vec::new(),
        proxy_jump: None,
        source: PathBuf::from("config"),
        line: 1,
    }
}

#[test]
fn fuzzy_filter_matches_alias_hostname_and_user() {
    let mut app = App::new(vec![
        host("prod-web", "10.0.0.1", "deploy"),
        host("staging", "stg.example.com", "ubuntu"),
        host("database", "db.internal", "postgres"),
    ]);

    app.set_query("prd");
    assert_eq!(app.visible_hosts()[0].alias, "prod-web");
    assert_eq!(app.visible_hosts().len(), 1);

    app.set_query("example");
    assert_eq!(app.visible_hosts()[0].alias, "staging");

    app.set_query("post");
    assert_eq!(app.visible_hosts()[0].alias, "database");
}

#[test]
fn selection_wraps_and_is_clamped_when_filter_changes() {
    let mut app = App::new(vec![
        host("alpha", "alpha.example.com", "a"),
        host("beta", "beta.example.com", "b"),
    ]);

    app.select_previous();
    assert_eq!(
        app.selected_host().map(|host| host.alias.as_str()),
        Some("beta")
    );
    app.select_next();
    assert_eq!(
        app.selected_host().map(|host| host.alias.as_str()),
        Some("alpha")
    );

    app.select_next();
    app.set_query("alpha");
    assert_eq!(
        app.selected_host().map(|host| host.alias.as_str()),
        Some("alpha")
    );
}

#[test]
fn fuzzy_filter_ranks_the_tighter_match_first() {
    let mut app = App::new(vec![
        host("production-database", "db.example.com", "ops"),
        host("prod", "prod.example.com", "deploy"),
    ]);

    app.set_query("prod");

    assert_eq!(app.visible_hosts()[0].alias, "prod");
}

#[test]
fn fuzzy_filter_is_case_insensitive_and_normalizes_unicode() {
    let mut app = App::new(vec![host("Café-Prod", "cafe.example.com", "Déployer")]);

    app.set_query("CAFE");

    assert_eq!(app.visible_hosts()[0].alias, "Café-Prod");
}

#[test]
fn filtering_selects_the_best_match_after_each_query_change() {
    let mut app = App::new(vec![
        host("prod-web", "10.0.0.1", "deploy"),
        host("dev-api", "dev.example.com", "developer"),
        host("dev-worker", "dev.example.com", "developer"),
    ]);

    app.set_query("d");
    assert_eq!(
        app.selected_host().map(|host| host.alias.as_str()),
        Some("dev-api")
    );
    app.set_query("dev");

    assert_eq!(
        app.selected_host().map(|host| host.alias.as_str()),
        Some("dev-api")
    );
}
