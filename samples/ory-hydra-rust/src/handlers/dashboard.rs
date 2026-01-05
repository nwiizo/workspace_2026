use crate::error::AppError;
use axum::{extract::Query, response::Html};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub email: Option<String>,
    pub role: Option<String>,
}

/// GET /dashboard - Dashboard page after login
pub async fn show_dashboard(Query(query): Query<DashboardQuery>) -> Result<Html<String>, AppError> {
    let email = query.email.unwrap_or_else(|| "Guest".to_string());
    let role = query.role.unwrap_or_else(|| "unknown".to_string());

    let (role_display, role_color, role_desc) = match role.as_str() {
        "platform_admin" => ("Platform Admin", "#e74c3c", "Full system access"),
        "tenant_admin" => ("Tenant Admin", "#9b59b6", "Manage your store"),
        "tenant_staff" => ("Staff", "#3498db", "Process orders"),
        "customer" => ("Customer", "#27ae60", "Shop and order"),
        _ => ("User", "#95a5a6", "Welcome"),
    };

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Dashboard - Multi-tenant EC Platform</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f5f6fa;
            min-height: 100vh;
        }}
        .navbar {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            padding: 15px 30px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        .navbar h1 {{ color: white; font-size: 20px; }}
        .user-info {{ display: flex; align-items: center; gap: 15px; }}
        .user-email {{ color: rgba(255,255,255,0.9); font-size: 14px; }}
        .role-badge {{
            background: {role_color};
            color: white;
            padding: 5px 12px;
            border-radius: 20px;
            font-size: 12px;
            font-weight: 600;
        }}
        .logout-btn {{
            background: rgba(255,255,255,0.2);
            color: white;
            border: none;
            padding: 8px 16px;
            border-radius: 5px;
            cursor: pointer;
            text-decoration: none;
            font-size: 14px;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 30px; }}
        .welcome-card {{
            background: white;
            border-radius: 10px;
            padding: 30px;
            margin-bottom: 30px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.05);
        }}
        .welcome-card h2 {{ color: #333; margin-bottom: 10px; }}
        .welcome-card p {{ color: #666; }}
        .cards-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 20px;
        }}
        .card {{
            background: white;
            border-radius: 10px;
            padding: 25px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.05);
            transition: transform 0.2s;
        }}
        .card:hover {{ transform: translateY(-5px); }}
        .card-icon {{
            width: 50px; height: 50px;
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 24px;
            margin-bottom: 15px;
        }}
        .card h3 {{ color: #333; margin-bottom: 10px; }}
        .card p {{ color: #666; font-size: 14px; margin-bottom: 15px; }}
        .card-link {{ color: #667eea; text-decoration: none; font-weight: 500; }}
        .icon-products {{ background: #e8f5e9; }}
        .icon-orders {{ background: #e3f2fd; }}
        .icon-users {{ background: #fce4ec; }}
        .icon-settings {{ background: #fff3e0; }}
        .api-section {{
            background: white;
            border-radius: 10px;
            padding: 25px;
            margin-top: 30px;
        }}
        .api-section h3 {{ color: #333; margin-bottom: 15px; }}
        .endpoint {{
            background: #f8f9fa;
            padding: 12px 15px;
            border-radius: 6px;
            margin-bottom: 10px;
            font-family: monospace;
            font-size: 13px;
        }}
        .method {{
            display: inline-block;
            padding: 3px 8px;
            border-radius: 4px;
            font-size: 11px;
            font-weight: 600;
            margin-right: 10px;
        }}
        .method-get {{ background: #27ae60; color: white; }}
        .method-post {{ background: #3498db; color: white; }}
    </style>
</head>
<body>
    <nav class="navbar">
        <h1>Multi-tenant EC Platform</h1>
        <div class="user-info">
            <span class="user-email">{email}</span>
            <span class="role-badge">{role_display}</span>
            <a href="/" class="logout-btn">Logout</a>
        </div>
    </nav>

    <div class="container">
        <div class="welcome-card">
            <h2>Welcome back!</h2>
            <p>{role_desc}</p>
        </div>

        <div class="cards-grid">
            <div class="card">
                <div class="card-icon icon-products">📦</div>
                <h3>Products</h3>
                <p>Manage your product catalog, inventory, and pricing.</p>
                <a href="/pages/products" class="card-link">View Products</a>
            </div>

            <div class="card">
                <div class="card-icon icon-orders">🛒</div>
                <h3>Orders</h3>
                <p>Track and manage customer orders and fulfillment.</p>
                <a href="/pages/orders" class="card-link">View Orders</a>
            </div>

            <div class="card">
                <div class="card-icon icon-users">👥</div>
                <h3>Tenants</h3>
                <p>Manage tenant stores on the platform.</p>
                <a href="/pages/tenants" class="card-link">View Tenants</a>
            </div>

            <div class="card">
                <div class="card-icon icon-settings">⚙️</div>
                <h3>Settings</h3>
                <p>Configure your account and preferences.</p>
                <a href="/dashboard" class="card-link">Settings</a>
            </div>
        </div>

        <div class="api-section">
            <h3>API Endpoints</h3>
            <div class="endpoint">
                <span class="method method-get">GET</span>
                /api/v1/tenants - List all tenants
            </div>
            <div class="endpoint">
                <span class="method method-post">POST</span>
                /api/v1/tenants - Create a new tenant
            </div>
            <div class="endpoint">
                <span class="method method-get">GET</span>
                /api/v1/{{tenant}}/products - List products
            </div>
            <div class="endpoint">
                <span class="method method-post">POST</span>
                /api/v1/{{tenant}}/orders - Create order
            </div>
        </div>
    </div>
</body>
</html>"##,
        role_color = role_color,
        email = email,
        role_display = role_display,
        role_desc = role_desc,
    );

    Ok(Html(html))
}
