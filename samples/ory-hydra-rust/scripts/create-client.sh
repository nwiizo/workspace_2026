#!/bin/bash
# Create a test OAuth2 client in Hydra

set -e

HYDRA_ADMIN_URL="${HYDRA_ADMIN_URL:-http://localhost:4445}"

echo "Creating OAuth2 client..."

curl -X POST "${HYDRA_ADMIN_URL}/admin/clients" \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "test-client",
    "client_name": "Test Application",
    "client_secret": "test-secret",
    "grant_types": ["authorization_code", "refresh_token"],
    "response_types": ["code"],
    "scope": "openid profile email offline_access",
    "redirect_uris": ["http://localhost:8080/callback"],
    "token_endpoint_auth_method": "client_secret_basic"
  }' | jq .

echo ""
echo "Client created successfully!"
echo ""
echo "You can now test the OAuth2 flow:"
echo "1. Open: http://localhost:4444/oauth2/auth?client_id=test-client&response_type=code&scope=openid+profile+email&redirect_uri=http://localhost:8080/callback&state=test-state"
echo "2. Login with: demo@example.com / password123"
echo "3. Grant consent"
echo "4. Copy the authorization code from the callback URL"
