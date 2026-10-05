use fundamentals_of_software_rust::orders::{Order, OrderAction, TransitionError};

fn main() -> Result<(), TransitionError> {
    let mut order = Order::default();
    println!("Created: {:?}", order.status());
    if let Err(error) = order.apply(OrderAction::Ship) {
        println!("Rejected: {error}; state remains {:?}", order.status());
    }
    order.apply(OrderAction::Pay)?;
    println!("Payment: {:?}", order.status());
    order.apply(OrderAction::Ship)?;
    println!("Shipment: {:?}", order.status());
    Ok(())
}
