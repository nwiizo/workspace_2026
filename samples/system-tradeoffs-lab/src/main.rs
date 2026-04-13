use system_tradeoffs_lab::{
    scenario_consistency_vs_latency, scenario_retry_with_idempotency,
    scenario_retry_without_idempotency,
};

fn main() {
    println!("== system-tradeoffs-lab ==");
    println!();

    let (result_without_idempotency, gateway_without_idempotency) =
        scenario_retry_without_idempotency();
    println!("scenario 1: retry without idempotency");
    println!("result: {:?}", result_without_idempotency);
    println!(
        "charges: {} / total_amount: {}",
        gateway_without_idempotency.total_charges(),
        gateway_without_idempotency.total_amount()
    );
    println!(
        "meaning: timeout after commit + retry can create a duplicate side effect"
    );
    println!();

    let (result_with_idempotency, gateway_with_idempotency) = scenario_retry_with_idempotency();
    println!("scenario 2: retry with idempotency");
    println!("result: {:?}", result_with_idempotency);
    println!(
        "charges: {} / total_amount: {}",
        gateway_with_idempotency.total_charges(),
        gateway_with_idempotency.total_amount()
    );
    println!("meaning: idempotency key makes retry safer");
    println!();

    let mut store = scenario_consistency_vs_latency();
    println!("scenario 3: consistency vs latency");
    println!(
        "primary stock right after purchase: {}",
        store.read_from_primary("book-ddia").unwrap().stock
    );
    println!(
        "replica stock before replication: {}",
        store.read_from_replica("book-ddia").unwrap().stock
    );
    store.replicate();
    println!(
        "replica stock after replication: {}",
        store.read_from_replica("book-ddia").unwrap().stock
    );
    println!("meaning: fast reads from replicas can be slightly stale");
}
