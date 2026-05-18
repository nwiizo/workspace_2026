//! ACM E2E tests against AWS official API spec.
//!
//! References:
//! - RequestCertificate:  <https://docs.aws.amazon.com/acm/latest/APIReference/API_RequestCertificate.html>
//! - DescribeCertificate: <https://docs.aws.amazon.com/acm/latest/APIReference/API_DescribeCertificate.html>
//! - ImportCertificate:   <https://docs.aws.amazon.com/acm/latest/APIReference/API_ImportCertificate.html>

mod common;

use aws_sdk_acm::primitives::Blob;
use aws_sdk_acm::types::CertificateStatus;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn e2e_acm_request_certificate_returns_arn() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let acm = aws_sdk_acm::Client::new(&cfg);

    let res = acm
        .request_certificate()
        .domain_name("kuroko.test")
        .send()
        .await
        .unwrap();
    let arn = res.certificate_arn().unwrap();
    assert!(arn.starts_with("arn:aws:acm:"));
    assert!(arn.contains(":certificate/"));
}

#[tokio::test]
async fn e2e_acm_describe_returns_issued_status() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let acm = aws_sdk_acm::Client::new(&cfg);

    let arn = acm
        .request_certificate()
        .domain_name("kuroko.test")
        .subject_alternative_names("www.kuroko.test")
        .send()
        .await
        .unwrap()
        .certificate_arn()
        .unwrap()
        .to_string();
    let desc = acm
        .describe_certificate()
        .certificate_arn(&arn)
        .send()
        .await
        .unwrap();
    let cert = desc.certificate().unwrap();
    assert_eq!(cert.status(), Some(&CertificateStatus::Issued));
    assert_eq!(cert.domain_name(), Some("kuroko.test"));
    assert!(
        cert.subject_alternative_names()
            .contains(&"www.kuroko.test".to_string())
    );
}

#[tokio::test]
async fn e2e_acm_list_certificates() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let acm = aws_sdk_acm::Client::new(&cfg);

    for d in ["a.test", "b.test", "c.test"] {
        acm.request_certificate()
            .domain_name(d)
            .send()
            .await
            .unwrap();
    }
    let res = acm.list_certificates().send().await.unwrap();
    assert_eq!(res.certificate_summary_list().len(), 3);
}

#[tokio::test]
async fn e2e_acm_delete_certificate() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let acm = aws_sdk_acm::Client::new(&cfg);

    let arn = acm
        .request_certificate()
        .domain_name("doomed.test")
        .send()
        .await
        .unwrap()
        .certificate_arn()
        .unwrap()
        .to_string();
    acm.delete_certificate()
        .certificate_arn(&arn)
        .send()
        .await
        .unwrap();
    let err = acm
        .describe_certificate()
        .certificate_arn(&arn)
        .send()
        .await;
    assert!(err.is_err(), "describe must fail after delete");
}

#[tokio::test]
async fn e2e_acm_import_certificate_then_get() {
    let srv = common::spawn().await;
    let cfg = common::aws_config(&srv.endpoint).await;
    let acm = aws_sdk_acm::Client::new(&cfg);

    let cert_pem = "-----BEGIN CERTIFICATE-----\nSubject: CN=mine.kuroko.test, O=kuroko\n-----END CERTIFICATE-----\n";
    let key_pem = "-----BEGIN PRIVATE KEY-----\nstub\n-----END PRIVATE KEY-----\n";

    let res = acm
        .import_certificate()
        .certificate(Blob::new(cert_pem.as_bytes().to_vec()))
        .private_key(Blob::new(key_pem.as_bytes().to_vec()))
        .send()
        .await
        .unwrap();
    let arn = res.certificate_arn().unwrap().to_string();

    let got = acm
        .get_certificate()
        .certificate_arn(&arn)
        .send()
        .await
        .unwrap();
    assert!(got.certificate().unwrap().contains("BEGIN CERTIFICATE"));
}
