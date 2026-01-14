#!/bin/bash
set -e

echo "=== Ory Kratos + Hydra Integration Test ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

KRATOS_PUBLIC_URL="http://localhost:4433"
KRATOS_ADMIN_URL="http://localhost:4434"
HYDRA_PUBLIC_URL="http://localhost:4444"
HYDRA_ADMIN_URL="http://localhost:4445"
UI_URL="http://localhost:4455"

# Wait for services to be ready
echo "Waiting for services..."
for i in {1..30}; do
    if curl -s "${KRATOS_PUBLIC_URL}/health/ready" > /dev/null 2>&1 && \
       curl -s "${HYDRA_PUBLIC_URL}/health/ready" > /dev/null 2>&1; then
        echo -e "${GREEN}Services are ready!${NC}"
        break
    fi
    echo "Waiting... ($i/30)"
    sleep 2
done

# 1. Check Kratos health
echo ""
echo "=== 1. Kratos Health Check ==="
KRATOS_HEALTH=$(curl -s "${KRATOS_PUBLIC_URL}/health/ready")
echo "Kratos: ${KRATOS_HEALTH}"

# 2. Check Hydra health
echo ""
echo "=== 2. Hydra Health Check ==="
HYDRA_HEALTH=$(curl -s "${HYDRA_PUBLIC_URL}/health/ready")
echo "Hydra: ${HYDRA_HEALTH}"

# 3. Create OAuth2 client
echo ""
echo "=== 3. Create OAuth2 Client ==="
CLIENT_RESPONSE=$(curl -s -X POST "${HYDRA_ADMIN_URL}/admin/clients" \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "test-client",
    "client_secret": "test-secret",
    "grant_types": ["authorization_code", "refresh_token"],
    "response_types": ["code"],
    "scope": "openid profile email",
    "redirect_uris": ["http://localhost:8080/callback"]
  }')

if echo "${CLIENT_RESPONSE}" | grep -q "client_id"; then
    echo -e "${GREEN}OAuth2 client created successfully${NC}"
    echo "Client ID: test-client"
else
    echo -e "${YELLOW}Client may already exist or error occurred${NC}"
    echo "${CLIENT_RESPONSE}"
fi

# 4. List identities
echo ""
echo "=== 4. List Kratos Identities ==="
IDENTITIES=$(curl -s "${KRATOS_ADMIN_URL}/admin/identities")
echo "Identities: ${IDENTITIES}"

# 5. Show OAuth2 authorization URL
echo ""
echo "=== 5. OAuth2 Authorization Flow ==="
echo ""
echo -e "${YELLOW}To test the full OAuth2 flow:${NC}"
echo ""
echo "1. Register a user:"
echo "   ${UI_URL}/registration"
echo ""
echo "2. Start OAuth2 authorization:"
AUTH_URL="${HYDRA_PUBLIC_URL}/oauth2/auth?client_id=test-client&response_type=code&scope=openid+profile+email&redirect_uri=http://localhost:8080/callback&state=test-state"
echo "   ${AUTH_URL}"
echo ""
echo "3. After login, exchange the code for tokens:"
echo "   curl -X POST ${HYDRA_PUBLIC_URL}/oauth2/token \\"
echo "     -u 'test-client:test-secret' \\"
echo "     -d 'grant_type=authorization_code&code=<CODE>&redirect_uri=http://localhost:8080/callback'"
echo ""

# 6. Show Kratos self-service URLs
echo ""
echo "=== 6. Kratos Self-Service URLs ==="
echo "Registration: ${UI_URL}/registration"
echo "Login:        ${UI_URL}/login"
echo "Settings:     ${UI_URL}/settings"
echo "Recovery:     ${UI_URL}/recovery"
echo "Verification: ${UI_URL}/verification"
echo ""

echo -e "${GREEN}=== Test script completed ===${NC}"
