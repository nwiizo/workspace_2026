use crate::error::AppError;
use crate::state::AppState;
use axum::{
    Form,
    extract::State,
    response::{Html, Redirect},
};
use serde::Deserialize;
use std::sync::Arc;

/// GET /pages/tenants - View tenants page
pub async fn tenants_page(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let tenants = state.tenant.list(100, 0).await?;

    let tenant_rows: Vec<String> = tenants
        .iter()
        .map(|t| {
            format!(
                r#"<tr>
                    <td>{}</td>
                    <td><strong>{}</strong></td>
                    <td><code>{}</code></td>
                    <td><span class="badge badge-{}">{}</span></td>
                    <td>{}</td>
                </tr>"#,
                &t.id.to_string()[..8],
                t.name,
                t.slug,
                t.status.to_lowercase(),
                t.status,
                t.created_at.format("%Y-%m-%d %H:%M")
            )
        })
        .collect();

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Tenants - DONADONA</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f6fa; }}
        .navbar {{
            background: linear-gradient(135deg, #EF4444 0%, #DC2626 100%);
            padding: 15px 30px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .navbar h1 {{ color: white; font-size: 20px; }}
        .navbar a {{ color: white; text-decoration: none; }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 30px; }}
        .header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }}
        .header h2 {{ color: #333; }}
        .btn {{
            background: linear-gradient(135deg, #EF4444 0%, #DC2626 100%);
            color: white;
            padding: 10px 20px;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            text-decoration: none;
            font-weight: 500;
        }}
        table {{ width: 100%; background: white; border-radius: 10px; overflow: hidden; box-shadow: 0 2px 10px rgba(0,0,0,0.05); }}
        th, td {{ padding: 15px; text-align: left; border-bottom: 1px solid #eee; }}
        th {{ background: #f8f9fa; font-weight: 600; color: #333; }}
        .badge {{ padding: 4px 12px; border-radius: 20px; font-size: 12px; font-weight: 600; }}
        .badge-active {{ background: #d4edda; color: #155724; }}
        .badge-inactive {{ background: #f8d7da; color: #721c24; }}
        .empty {{ text-align: center; padding: 40px; color: #666; }}
        code {{ background: #f1f1f1; padding: 2px 8px; border-radius: 4px; }}
    </style>
</head>
<body>
    <nav class="navbar">
        <h1>DONADONA</h1>
        <a href="/dashboard">Back to Dashboard</a>
    </nav>
    <div class="container">
        <div class="header">
            <h2>Tenants ({} total)</h2>
            <a href="/pages/tenants/new" class="btn">+ New Tenant</a>
        </div>
        <table>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Name</th>
                    <th>Slug</th>
                    <th>Status</th>
                    <th>Created</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>
</body>
</html>"##,
        tenants.len(),
        if tenant_rows.is_empty() {
            r#"<tr><td colspan="5" class="empty">No tenants yet. Create one to get started!</td></tr>"#.to_string()
        } else {
            tenant_rows.join("\n")
        }
    );

    Ok(Html(html))
}

/// GET /pages/tenants/new - New tenant form
pub async fn new_tenant_page() -> Html<String> {
    Html(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>New Tenant - DONADONA</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f6fa; }
        .navbar { background: linear-gradient(135deg, #EF4444 0%, #DC2626 100%); padding: 15px 30px; }
        .navbar h1 { color: white; font-size: 20px; }
        .container { max-width: 600px; margin: 0 auto; padding: 30px; }
        .card { background: white; border-radius: 10px; padding: 30px; box-shadow: 0 2px 10px rgba(0,0,0,0.05); }
        h2 { color: #333; margin-bottom: 20px; }
        .form-group { margin-bottom: 20px; }
        label { display: block; margin-bottom: 8px; font-weight: 500; color: #333; }
        input { width: 100%; padding: 12px; border: 2px solid #e1e1e1; border-radius: 6px; font-size: 16px; }
        input:focus { outline: none; border-color: #EF4444; }
        .btn { background: linear-gradient(135deg, #EF4444 0%, #DC2626 100%); color: white; padding: 14px 30px; border: none; border-radius: 6px; cursor: pointer; font-size: 16px; font-weight: 600; width: 100%; }
        .back { display: block; text-align: center; margin-top: 15px; color: #EF4444; text-decoration: none; }
    </style>
</head>
<body>
    <nav class="navbar"><h1>DONADONA</h1></nav>
    <div class="container">
        <div class="card">
            <h2>Create New Tenant</h2>
            <form action="/pages/tenants/create" method="POST">
                <div class="form-group">
                    <label for="name">Tenant Name</label>
                    <input type="text" id="name" name="name" required placeholder="My Team">
                </div>
                <div class="form-group">
                    <label for="slug">Slug</label>
                    <input type="text" id="slug" name="slug" required placeholder="my-team" pattern="[a-z0-9-]+">
                </div>
                <button type="submit" class="btn">Create Tenant</button>
            </form>
            <a href="/pages/tenants" class="back">Back to Tenants</a>
        </div>
    </div>
</body>
</html>"##.to_string())
}

#[derive(Deserialize)]
pub struct CreateTenantForm {
    pub name: String,
    pub slug: String,
}

/// POST /pages/tenants/create - Create tenant
pub async fn create_tenant_page(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CreateTenantForm>,
) -> Result<Redirect, AppError> {
    use crate::models::CreateTenantRequest;

    let request = CreateTenantRequest {
        slug: form.slug,
        name: form.name,
        plan: None, // defaults to "free"
    };

    // TenantService.create handles both DB insert and schema creation
    state.tenant.create(request).await?;

    Ok(Redirect::to("/pages/tenants"))
}
