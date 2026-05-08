use anyhow::{Context, Result};
use aws_credential_types::Credentials;
use aws_sdk_s3 as s3;
use aws_sdk_sqs as sqs;

async fn check(label: &str, endpoint: &str) -> Result<()> {
    let creds = Credentials::new("test", "test", None, None, "static");
    let cfg = aws_config::from_env()
        .endpoint_url(endpoint)
        .region("us-east-1")
        .credentials_provider(creds)
        .load()
        .await;
    let s3_cfg = s3::config::Builder::from(&cfg).force_path_style(true).build();
    let s3c = s3::Client::from_conf(s3_cfg);
    let bucket = format!("sdk-{}", label);
    match async {
        s3c.create_bucket().bucket(&bucket).send().await?;
        s3c.put_object()
            .bucket(&bucket)
            .key("hello.txt")
            .body(s3::primitives::ByteStream::from_static(b"hi from aws-sdk-rust"))
            .send()
            .await?;
        let got = s3c.get_object().bucket(&bucket).key("hello.txt").send().await?;
        let body = got.body.collect().await?.into_bytes();
        assert_eq!(&body[..], b"hi from aws-sdk-rust");
        Ok::<usize, anyhow::Error>(body.len())
    }
    .await
    {
        Ok(len) => println!("[{}] s3 put/get OK (len={})", label, len),
        Err(err) => println!("[{}] s3 put/get FAILED: {err:#}", label),
    }

    let sqsc = sqs::Client::new(&cfg);
    match async {
        let q = sqsc
            .create_queue()
            .queue_name(format!("sdk-{}", label))
            .send()
            .await?;
        let url = q.queue_url.context("missing queue URL")?;
        sqsc.send_message()
            .queue_url(&url)
            .message_body("rust-msg")
            .send()
            .await?;
        let recv = sqsc
            .receive_message()
            .queue_url(&url)
            .wait_time_seconds(1)
            .send()
            .await?;
        let msgs = recv.messages.unwrap_or_default();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].body.as_deref(), Some("rust-msg"));
        Ok::<(), anyhow::Error>(())
    }
    .await
    {
        Ok(()) => println!("[{}] sqs send/receive OK", label),
        Err(err) => println!("[{}] sqs send/receive FAILED: {err:#}", label),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    check("kumo", "http://localhost:4566").await?;
    println!("all good");
    Ok(())
}
