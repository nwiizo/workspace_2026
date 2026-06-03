#!/bin/bash
# Create a test OAuth2 client in Hydra

set -e

HYDRA_ADMIN_URL="${HYDRA_ADMIN_URL:-http://localhost:4445}"

echo "Creating OAuth2 client..."

curl -X PUT "${HYDRA_ADMIN_URL}/admin/clients/demo-client" \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "demo-client",
    "client_name": "DONADONA BFF",
    "client_secret": "demo-secret",
    "grant_types": ["authorization_code", "refresh_token"],
    "response_types": ["code"],
    "scope": "openid profile email offline_access",
    "redirect_uris": ["http://localhost:3000/api/bff/callback"],
    "token_endpoint_auth_method": "client_secret_basic"
  }' | jq .

echo ""
echo "Client created successfully!"
echo ""
echo "You can now test the OAuth2 flow:"
echo "1. Open: http://localhost:3000/api/bff/login?redirect=/dashboard"
echo "2. Login with: demo@example.com / password123"
echo "3. Grant consent"
echo "4. The BFF stores tokens server-side and redirects back to the SPA"
