use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderRequest {
    pub request_id: String,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargeRecord {
    pub request_id: String,
    pub amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayStep {
    TimeoutAfterCommit,
    TemporaryFailure,
    Success,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayError {
    Timeout,
    TemporaryFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutError {
    ExhaustedRetries,
}

#[derive(Debug, Clone)]
pub struct FakePaymentGateway {
    steps: Vec<GatewayStep>,
    charges: Vec<ChargeRecord>,
}

impl FakePaymentGateway {
    pub fn new(steps: Vec<GatewayStep>) -> Self {
        Self {
            steps,
            charges: Vec::new(),
        }
    }

    pub fn charge(&mut self, request: &OrderRequest) -> Result<(), GatewayError> {
        let step = if self.steps.is_empty() {
            GatewayStep::Success
        } else {
            self.steps.remove(0)
        };

        match step {
            GatewayStep::TimeoutAfterCommit => {
                self.charges.push(ChargeRecord {
                    request_id: request.request_id.clone(),
                    amount: request.amount,
                });
                Err(GatewayError::Timeout)
            }
            GatewayStep::TemporaryFailure => Err(GatewayError::TemporaryFailure),
            GatewayStep::Success => {
                self.charges.push(ChargeRecord {
                    request_id: request.request_id.clone(),
                    amount: request.amount,
                });
                Ok(())
            }
        }
    }

    pub fn total_charges(&self) -> usize {
        self.charges.len()
    }

    pub fn total_amount(&self) -> u64 {
        self.charges.iter().map(|charge| charge.amount).sum()
    }

    pub fn charges(&self) -> &[ChargeRecord] {
        &self.charges
    }
}

#[derive(Debug, Clone)]
pub struct CheckoutService {
    max_retries: usize,
    use_idempotency: bool,
    processed: HashMap<String, ChargeRecord>,
}

impl CheckoutService {
    pub fn new(max_retries: usize, use_idempotency: bool) -> Self {
        Self {
            max_retries,
            use_idempotency,
            processed: HashMap::new(),
        }
    }

    pub fn checkout(
        &mut self,
        gateway: &mut FakePaymentGateway,
        request: OrderRequest,
    ) -> Result<(), CheckoutError> {
        if self.use_idempotency && self.processed.contains_key(&request.request_id) {
            return Ok(());
        }

        for attempt in 0..=self.max_retries {
            match gateway.charge(&request) {
                Ok(()) => {
                    if self.use_idempotency {
                        self.processed.insert(
                            request.request_id.clone(),
                            ChargeRecord {
                                request_id: request.request_id.clone(),
                                amount: request.amount,
                            },
                        );
                    }
                    return Ok(());
                }
                Err(GatewayError::Timeout) => {
                    if self.use_idempotency {
                        self.processed
                            .entry(request.request_id.clone())
                            .or_insert(ChargeRecord {
                                request_id: request.request_id.clone(),
                                amount: request.amount,
                            });
                        return Ok(());
                    }

                    if attempt == self.max_retries {
                        return Err(CheckoutError::ExhaustedRetries);
                    }
                }
                Err(GatewayError::TemporaryFailure) => {
                    if attempt == self.max_retries {
                        return Err(CheckoutError::ExhaustedRetries);
                    }
                }
            }
        }

        Err(CheckoutError::ExhaustedRetries)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub sku: String,
    pub stock: u32,
}

#[derive(Debug, Clone)]
pub struct InventoryStore {
    primary: HashMap<String, Product>,
    replica: HashMap<String, Product>,
}

impl InventoryStore {
    pub fn new(initial: &[Product]) -> Self {
        let primary = initial
            .iter()
            .cloned()
            .map(|product| (product.sku.clone(), product))
            .collect::<HashMap<_, _>>();
        let replica = primary.clone();
        Self { primary, replica }
    }

    pub fn purchase_on_primary(&mut self, sku: &str, quantity: u32) {
        if let Some(product) = self.primary.get_mut(sku) {
            product.stock = product.stock.saturating_sub(quantity);
        }
    }

    pub fn read_from_primary(&self, sku: &str) -> Option<&Product> {
        self.primary.get(sku)
    }

    pub fn read_from_replica(&self, sku: &str) -> Option<&Product> {
        self.replica.get(sku)
    }

    pub fn replicate(&mut self) {
        self.replica = self.primary.clone();
    }
}

pub fn scenario_retry_without_idempotency() -> (Result<(), CheckoutError>, FakePaymentGateway) {
    let request = OrderRequest {
        request_id: "order-001".to_string(),
        amount: 5_000,
    };
    let mut gateway = FakePaymentGateway::new(vec![
        GatewayStep::TimeoutAfterCommit,
        GatewayStep::Success,
    ]);
    let mut service = CheckoutService::new(1, false);
    let result = service.checkout(&mut gateway, request);
    (result, gateway)
}

pub fn scenario_retry_with_idempotency() -> (Result<(), CheckoutError>, FakePaymentGateway) {
    let request = OrderRequest {
        request_id: "order-001".to_string(),
        amount: 5_000,
    };
    let mut gateway = FakePaymentGateway::new(vec![
        GatewayStep::TimeoutAfterCommit,
        GatewayStep::Success,
    ]);
    let mut service = CheckoutService::new(1, true);
    let result = service.checkout(&mut gateway, request);
    (result, gateway)
}

pub fn scenario_consistency_vs_latency() -> InventoryStore {
    let mut store = InventoryStore::new(&[Product {
        sku: "book-ddia".to_string(),
        stock: 3,
    }]);
    store.purchase_on_primary("book-ddia", 1);
    store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_without_idempotency_can_duplicate_the_charge() {
        let (result, gateway) = scenario_retry_without_idempotency();

        assert_eq!(result, Ok(()));
        assert_eq!(gateway.total_charges(), 2);
        assert_eq!(gateway.total_amount(), 10_000);
    }

    #[test]
    fn idempotency_stops_the_second_charge_after_timeout() {
        let (result, gateway) = scenario_retry_with_idempotency();

        assert_eq!(result, Ok(()));
        assert_eq!(gateway.total_charges(), 1);
        assert_eq!(gateway.total_amount(), 5_000);
    }

    #[test]
    fn replica_can_be_fast_but_stale_until_replication_happens() {
        let mut store = scenario_consistency_vs_latency();

        let primary = store.read_from_primary("book-ddia").unwrap();
        let replica_before = store.read_from_replica("book-ddia").unwrap();

        assert_eq!(primary.stock, 2);
        assert_eq!(replica_before.stock, 3);

        store.replicate();

        let replica_after = store.read_from_replica("book-ddia").unwrap();
        assert_eq!(replica_after.stock, 2);
    }
}
