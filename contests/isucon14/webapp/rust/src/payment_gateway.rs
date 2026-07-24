use crate::Error;
use std::time::Instant;

#[derive(Debug, thiserror::Error)]
pub enum PaymentGatewayError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("[POST /payments] unexpected status code ({0})")]
    PostPayment(reqwest::StatusCode),
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct PaymentGatewayPostPaymentRequest {
    pub(crate) amount: i32,
}

#[derive(Debug, Default)]
pub(crate) struct PaymentGatewayDiagnostic {
    attempts: u32,
    request_us: u64,
    retry_sleep_us: u64,
    network_errors: u32,
    conflict_errors: u32,
    server_errors: u32,
    other_status_errors: u32,
    terminal_status: Option<u16>,
    request_started_at: Option<Instant>,
    retry_sleep_started_at: Option<Instant>,
}

impl PaymentGatewayDiagnostic {
    fn elapsed_us(started_at: Instant) -> u64 {
        started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
    }

    fn request_started(&mut self) {
        self.attempts = self.attempts.saturating_add(1);
        self.request_started_at = Some(Instant::now());
    }

    fn request_finished(&mut self, result: &Result<(), PaymentGatewayError>) {
        if let Some(started_at) = self.request_started_at.take() {
            self.request_us = self.request_us.saturating_add(Self::elapsed_us(started_at));
        }
        match result {
            Ok(()) => self.terminal_status = Some(204),
            Err(PaymentGatewayError::Reqwest(_)) => {
                self.network_errors = self.network_errors.saturating_add(1);
                self.terminal_status = None;
            }
            Err(PaymentGatewayError::PostPayment(status)) => {
                self.terminal_status = Some(status.as_u16());
                if *status == reqwest::StatusCode::CONFLICT {
                    self.conflict_errors = self.conflict_errors.saturating_add(1);
                } else if status.is_server_error() {
                    self.server_errors = self.server_errors.saturating_add(1);
                } else {
                    self.other_status_errors = self.other_status_errors.saturating_add(1);
                }
            }
        }
    }

    fn retry_sleep_started(&mut self) {
        self.retry_sleep_started_at = Some(Instant::now());
    }

    fn retry_sleep_finished(&mut self) {
        if let Some(started_at) = self.retry_sleep_started_at.take() {
            self.retry_sleep_us = self
                .retry_sleep_us
                .saturating_add(Self::elapsed_us(started_at));
        }
    }

    pub(crate) fn attempts(&self) -> u32 {
        self.attempts
    }

    pub(crate) fn request_us(&self) -> u64 {
        self.request_started_at
            .map_or(self.request_us, |started_at| {
                self.request_us.saturating_add(Self::elapsed_us(started_at))
            })
    }

    pub(crate) fn retry_sleep_us(&self) -> u64 {
        self.retry_sleep_started_at
            .map_or(self.retry_sleep_us, |started_at| {
                self.retry_sleep_us
                    .saturating_add(Self::elapsed_us(started_at))
            })
    }

    pub(crate) fn network_errors(&self) -> u32 {
        self.network_errors
    }

    pub(crate) fn conflict_errors(&self) -> u32 {
        self.conflict_errors
    }

    pub(crate) fn server_errors(&self) -> u32 {
        self.server_errors
    }

    pub(crate) fn other_status_errors(&self) -> u32 {
        self.other_status_errors
    }

    pub(crate) fn terminal_status(&self) -> Option<u16> {
        self.terminal_status
    }
}

pub(crate) async fn request_payment_gateway_post_payment(
    client: &reqwest::Client,
    payment_gateway_url: &str,
    token: &str,
    idempotency_key: &str,
    param: &PaymentGatewayPostPaymentRequest,
    mut diagnostic: Option<&mut PaymentGatewayDiagnostic>,
) -> Result<(), Error> {
    // Network errors, a concurrent request with the same key (409), and 5xx
    // are transient. Other 4xx responses cannot succeed with the same
    // idempotency key and payload, so return them without holding the ride
    // transaction through five unnecessary retries.
    let mut retry = 0;

    loop {
        if let Some(diagnostic) = diagnostic.as_deref_mut() {
            diagnostic.request_started();
        }
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

        if let Some(diagnostic) = diagnostic.as_deref_mut() {
            diagnostic.request_finished(&result);
        }

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
                if let Some(diagnostic) = diagnostic.as_deref_mut() {
                    diagnostic.retry_sleep_started();
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if let Some(diagnostic) = diagnostic.as_deref_mut() {
                    diagnostic.retry_sleep_finished();
                }
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

        let mut diagnostic = PaymentGatewayDiagnostic::default();
        request_payment_gateway_post_payment(
            &reqwest::Client::new(),
            &format!("http://{address}"),
            "payment-token",
            "ride-1",
            &PaymentGatewayPostPaymentRequest { amount: 700 },
            Some(&mut diagnostic),
        )
        .await
        .unwrap();

        assert_eq!(diagnostic.attempts, 2);
        assert_eq!(diagnostic.server_errors, 1);
        assert_eq!(diagnostic.terminal_status, Some(204));
        assert!(diagnostic.request_us > 0);
        assert!(diagnostic.retry_sleep_us >= 100_000);

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
    async fn cancellation_preserves_the_in_flight_attempt() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_http_headers(&mut stream);
            std::thread::sleep(std::time::Duration::from_millis(200));
            let _ = write!(
                stream,
                "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
        });

        let mut diagnostic = PaymentGatewayDiagnostic::default();
        let result = tokio::time::timeout(
            tokio::time::Duration::from_millis(20),
            request_payment_gateway_post_payment(
                &reqwest::Client::new(),
                &format!("http://{address}"),
                "payment-token",
                "ride-1",
                &PaymentGatewayPostPaymentRequest { amount: 700 },
                Some(&mut diagnostic),
            ),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(diagnostic.attempts(), 1);
        assert!(diagnostic.request_us() >= 20_000);
        server.join().unwrap();
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

        let mut diagnostic = PaymentGatewayDiagnostic::default();
        let error = request_payment_gateway_post_payment(
            &reqwest::Client::new(),
            &format!("http://{address}"),
            "payment-token",
            "ride-1",
            &PaymentGatewayPostPaymentRequest { amount: 700 },
            Some(&mut diagnostic),
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
        assert_eq!(diagnostic.attempts, 1);
        assert_eq!(diagnostic.other_status_errors, 1);
        assert_eq!(diagnostic.terminal_status, Some(422));
    }
}
