use crate::Error;

#[derive(Debug, thiserror::Error)]
pub enum PaymentGatewayError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("[POST /payments] unexpected status code ({0})")]
    PostPayment(reqwest::StatusCode),
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentGatewayPostPaymentRequest {
    pub amount: i32,
}

pub async fn request_payment_gateway_post_payment(
    client: &reqwest::Client,
    payment_gateway_url: &str,
    token: &str,
    idempotency_key: &str,
    param: &PaymentGatewayPostPaymentRequest,
) -> Result<(), Error> {
    // Network errors, a concurrent request with the same key (409), and 5xx
    // are transient. Other 4xx responses cannot succeed with the same
    // idempotency key and payload, so return them without holding the ride
    // transaction through five unnecessary retries.
    let mut retry = 0;

    loop {
        let result: Result<(), PaymentGatewayError> = async {
            let res = client
                .post(format!("{payment_gateway_url}/payments"))
                .bearer_auth(token)
                .header("Idempotency-Key", idempotency_key)
                .json(param)
                .send()
                .await
                .map_err(PaymentGatewayError::Reqwest)?;

            if res.status() != reqwest::StatusCode::NO_CONTENT {
                return Err(PaymentGatewayError::PostPayment(res.status()));
            }
            Ok(())
        }
        .await;

        if let Err(err) = result {
            let retryable = match &err {
                PaymentGatewayError::Reqwest(_) => true,
                PaymentGatewayError::PostPayment(status) => {
                    *status == reqwest::StatusCode::CONFLICT || status.is_server_error()
                }
            };
            if !retryable {
                return Err(err.into());
            }
            if retry < 5 {
                retry += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            } else {
                return Err(err.into());
            }
        }
        break;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    fn read_http_headers(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let mut chunk = [0_u8; 1024];
            let read_length = stream.read(&mut chunk).unwrap();
            assert!(read_length > 0, "request ended before the HTTP headers");
            request.extend_from_slice(&chunk[..read_length]);
        }
        String::from_utf8_lossy(&request).into_owned()
    }

    #[tokio::test]
    async fn retry_reuses_the_same_idempotency_key_without_getting_payment_history() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            [500, 204]
                .into_iter()
                .map(|status| {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_http_headers(&mut stream);
                    let reason = if status == 204 {
                        "No Content"
                    } else {
                        "Internal Server Error"
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .unwrap();
                    request
                })
                .collect::<Vec<_>>()
        });

        request_payment_gateway_post_payment(
            &reqwest::Client::new(),
            &format!("http://{address}"),
            "payment-token",
            "ride-1",
            &PaymentGatewayPostPaymentRequest { amount: 700 },
        )
        .await
        .unwrap();

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        for request in requests {
            let request = request.to_ascii_lowercase();
            assert!(request.starts_with("post /payments http/1.1\r\n"));
            assert!(request.contains("\r\nidempotency-key: ride-1\r\n"));
            assert!(!request.starts_with("get "));
        }
    }

    #[tokio::test]
    async fn permanent_client_error_is_not_retried() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_http_headers(&mut stream);
            write!(
                stream,
                "HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        });

        let error = request_payment_gateway_post_payment(
            &reqwest::Client::new(),
            &format!("http://{address}"),
            "payment-token",
            "ride-1",
            &PaymentGatewayPostPaymentRequest { amount: 700 },
        )
        .await
        .unwrap_err();
        server.join().unwrap();

        assert!(matches!(
            error,
            Error::PaymentGateway(PaymentGatewayError::PostPayment(
                reqwest::StatusCode::UNPROCESSABLE_ENTITY
            ))
        ));
    }
}
