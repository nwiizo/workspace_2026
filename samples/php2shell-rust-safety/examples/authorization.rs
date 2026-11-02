#![forbid(unsafe_code)]

use php2shell_rust_safety::{Invoice, find_invoice_by_id, find_invoice_for_tenant};

fn main() {
    let invoices = [Invoice {
        id: 202,
        tenant_id: 2,
        description: "Tenant 2 private invoice",
    }];
    let authenticated_tenant = 1;

    let leaked = find_invoice_by_id(&invoices, 202);
    let authorized = find_invoice_for_tenant(&invoices, authenticated_tenant, 202);
    assert_eq!(leaked, Some(&invoices[0]));
    assert_eq!(authorized, None);

    println!("authenticated tenant: {authenticated_tenant}");
    println!("ID-only lookup: {leaked:?}");
    println!("tenant-scoped lookup: {authorized:?}");
}
