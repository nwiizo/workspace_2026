use fundamentals_of_software_rust::orders::{Order, OrderAction, OrderStatus};

#[test]
fn paid_order_can_be_shipped() {
    let mut order = Order::default();
    assert_eq!(order.status(), OrderStatus::Pending);
    order.apply(OrderAction::Pay).unwrap();
    assert_eq!(order.status(), OrderStatus::Paid);
    order.apply(OrderAction::Ship).unwrap();
    assert_eq!(order.status(), OrderStatus::Shipped);
}

#[test]
fn cancellation_is_terminal() {
    let mut order = Order::default();
    order.apply(OrderAction::Cancel).unwrap();
    for action in [OrderAction::Pay, OrderAction::Ship, OrderAction::Cancel] {
        assert!(order.apply(action).is_err());
        assert_eq!(order.status(), OrderStatus::Cancelled);
    }
}

#[test]
fn invalid_transitions_preserve_state_and_explain_the_failure() {
    let mut order = Order::default();
    let error = order.apply(OrderAction::Ship).unwrap_err();
    assert_eq!(error.status, OrderStatus::Pending);
    assert_eq!(error.action, OrderAction::Ship);
    assert_eq!(order.status(), OrderStatus::Pending);
    order.apply(OrderAction::Pay).unwrap();
    for action in [OrderAction::Pay, OrderAction::Cancel] {
        assert!(order.apply(action).is_err());
        assert_eq!(order.status(), OrderStatus::Paid);
    }
    order.apply(OrderAction::Ship).unwrap();
    for action in [OrderAction::Pay, OrderAction::Ship, OrderAction::Cancel] {
        assert!(order.apply(action).is_err());
        assert_eq!(order.status(), OrderStatus::Shipped);
    }
}
