use anyhow::Context;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use std::path::{Path, PathBuf};

pub struct CertPaths {
    pub cert: PathBuf,
    pub key: PathBuf,
}

pub fn ensure_self_signed(state_dir: &Path, host: &str) -> anyhow::Result<CertPaths> {
    std::fs::create_dir_all(state_dir)?;
    let cert_path = state_dir.join(format!("{}.cert.pem", sanitize(host)));
    let key_path = state_dir.join(format!("{}.key.pem", sanitize(host)));

    if cert_path.exists() && key_path.exists() {
        return Ok(CertPaths {
            cert: cert_path,
            key: key_path,
        });
    }

    let mut params = CertificateParams::default();
    params.distinguished_name = {
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, host);
        dn.push(DnType::OrganizationName, "remote-mcp-devkit");
        dn
    };

    let mut san = vec![SanType::DnsName(host.to_string().try_into()?)];
    if host != "localhost" {
        san.push(SanType::DnsName("localhost".to_string().try_into()?));
    }
    if let Ok(ip) = "127.0.0.1".parse::<std::net::IpAddr>() {
        san.push(SanType::IpAddress(ip));
    }
    if let Ok(ip) = "::1".parse::<std::net::IpAddr>() {
        san.push(SanType::IpAddress(ip));
    }
    params.subject_alt_names = san;

    let key_pair = KeyPair::generate().context("generate key pair")?;
    let cert = params.self_signed(&key_pair).context("self-sign cert")?;
    std::fs::write(&cert_path, cert.pem())?;
    std::fs::write(&key_path, key_pair.serialize_pem())?;

    Ok(CertPaths {
        cert: cert_path,
        key: key_path,
    })
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}
