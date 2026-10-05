use fundamentals_of_software_rust::config::{AppConfig, ConfigError};

#[test]
fn only_an_absent_port_uses_the_default() {
    assert_eq!(AppConfig::from_port(None).unwrap().port, 8080);
    assert_eq!(AppConfig::from_port(Some("3000")).unwrap().port, 3000);
    assert_eq!(AppConfig::from_port(Some("1")).unwrap().port, 1);
    assert_eq!(AppConfig::from_port(Some("65535")).unwrap().port, 65535);
}

#[test]
fn invalid_configuration_is_not_silently_replaced_by_a_default() {
    for input in ["", "0", "-1", "65536", "abc", " 8080", "8080 "] {
        assert_eq!(
            AppConfig::from_port(Some(input)),
            Err(ConfigError::InvalidPort),
            "{input:?}"
        );
    }
}
