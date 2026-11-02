use php2shell_rust_safety::{Invoice, find_invoice_by_id, find_invoice_for_tenant};

fn invoices() -> [Invoice; 2] {
    [
        Invoice {
            id: 101,
            tenant_id: 1,
            description: "Tenant 1 private invoice",
        },
        Invoice {
            id: 202,
            tenant_id: 2,
            description: "Tenant 2 private invoice",
        },
    ]
}

#[test]
fn safe_lookup_without_authorization_exposes_another_tenants_invoice() {
    let invoices = invoices();
    let leaked = find_invoice_by_id(&invoices, 202).expect("invoice 202 exists");
    assert_eq!(leaked.tenant_id, 2);
    assert_eq!(leaked.description, "Tenant 2 private invoice");
}

#[test]
fn authorized_lookup_rejects_another_tenants_invoice() {
    assert_eq!(find_invoice_for_tenant(&invoices(), 1, 202), None);
}

#[test]
fn authorized_lookup_preserves_access_to_the_owners_invoice() {
    let invoices = invoices();
    assert_eq!(
        find_invoice_for_tenant(&invoices, 1, 101),
        Some(&invoices[0])
    );
    assert_eq!(
        find_invoice_for_tenant(&invoices, 2, 202),
        Some(&invoices[1])
    );
}

#[test]
fn authorized_lookup_returns_none_for_a_missing_invoice() {
    assert_eq!(find_invoice_for_tenant(&invoices(), 1, 303), None);
}
