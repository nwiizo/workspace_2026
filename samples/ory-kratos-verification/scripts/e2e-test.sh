#!/bin/bash
set -e

echo "=========================================="
echo "  E2E Test: Ory Kratos + Hydra"
echo "=========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

KRATOS_PUBLIC="http://localhost:4433"
HYDRA_PUBLIC="http://localhost:4444"
HYDRA_ADMIN="http://localhost:4445"

TEST_EMAIL="e2etest-$(date +%s)@example.com"
# Use a strong random password to avoid HIBP detection
TEST_PASSWORD="Kratos$(date +%s)E2E!Xk9#mN"

echo "Test email: $TEST_EMAIL"
echo ""

# ==========================================
# 1. Registration Flow
# ==========================================
echo "=== 1. Registration Flow ==="
echo ""

echo "1.1 Initialize registration flow..."
REG_FLOW=$(curl -s -X GET "$KRATOS_PUBLIC/self-service/registration/api")
REG_FLOW_ID=$(echo "$REG_FLOW" | jq -r '.id')
echo "    Flow ID: $REG_FLOW_ID"

echo ""
echo "1.2 Submit registration..."
REG_RESULT=$(curl -s -X POST "$KRATOS_PUBLIC/self-service/registration?flow=$REG_FLOW_ID" \
  -H "Content-Type: application/json" \
  -d "{
    \"method\": \"password\",
    \"password\": \"$TEST_PASSWORD\",
    \"traits\": {
      \"email\": \"$TEST_EMAIL\",
      \"name\": {
        \"first\": \"E2E\",
        \"last\": \"Test\"
      }
    }
  }")

if echo "$REG_RESULT" | jq -e '.identity' > /dev/null 2>&1; then
  IDENTITY_ID=$(echo "$REG_RESULT" | jq -r '.identity.id')
  echo -e "    ${GREEN}Registration successful!${NC}"
  echo "    Identity ID: $IDENTITY_ID"
  echo "    Email: $(echo "$REG_RESULT" | jq -r '.identity.traits.email')"
  SESSION_TOKEN=$(echo "$REG_RESULT" | jq -r '.session_token // empty')
  if [ -n "$SESSION_TOKEN" ]; then
    echo "    Session Token: ${SESSION_TOKEN:0:20}..."
  fi
else
  echo -e "    ${RED}Registration failed${NC}"
  echo "$REG_RESULT" | jq .
  exit 1
fi

echo ""

# ==========================================
# 2. Login Flow
# ==========================================
echo "=== 2. Login Flow ==="
echo ""

echo "2.1 Initialize login flow..."
LOGIN_FLOW=$(curl -s -X GET "$KRATOS_PUBLIC/self-service/login/api")
LOGIN_FLOW_ID=$(echo "$LOGIN_FLOW" | jq -r '.id')
echo "    Flow ID: $LOGIN_FLOW_ID"

echo ""
echo "2.2 Submit login..."
LOGIN_RESULT=$(curl -s -X POST "$KRATOS_PUBLIC/self-service/login?flow=$LOGIN_FLOW_ID" \
  -H "Content-Type: application/json" \
  -d "{
    \"method\": \"password\",
    \"identifier\": \"$TEST_EMAIL\",
    \"password\": \"$TEST_PASSWORD\"
  }")

if echo "$LOGIN_RESULT" | jq -e '.session' > /dev/null 2>&1; then
  SESSION_ID=$(echo "$LOGIN_RESULT" | jq -r '.session.id')
  SESSION_TOKEN=$(echo "$LOGIN_RESULT" | jq -r '.session_token')
  echo -e "    ${GREEN}Login successful!${NC}"
  echo "    Session ID: $SESSION_ID"
  echo "    Session Token: ${SESSION_TOKEN:0:20}..."
else
  echo -e "    ${RED}Login failed${NC}"
  echo "$LOGIN_RESULT" | jq .
  exit 1
fi

echo ""

# ==========================================
# 3. Verify Session (whoami)
# ==========================================
echo "=== 3. Session Verification ==="
echo ""

echo "3.1 Check session (whoami)..."
WHOAMI=$(curl -s -X GET "$KRATOS_PUBLIC/sessions/whoami" \
  -H "Authorization: Bearer $SESSION_TOKEN")

if echo "$WHOAMI" | jq -e '.identity' > /dev/null 2>&1; then
  echo -e "    ${GREEN}Session valid!${NC}"
  echo "    Identity: $(echo "$WHOAMI" | jq -r '.identity.traits.email')"
  echo "    Active: $(echo "$WHOAMI" | jq -r '.active')"
else
  echo -e "    ${RED}Session invalid${NC}"
  echo "$WHOAMI" | jq .
fi

echo ""

# ==========================================
# 4. OAuth2 Client Setup
# ==========================================
echo "=== 4. OAuth2 Client Setup ==="
echo ""

echo "4.1 Create/verify OAuth2 client..."
CLIENT_ID="e2e-test-client-$(date +%s)"
CLIENT_SECRET="e2e-test-secret"

CLIENT_RESULT=$(curl -s -X POST "$HYDRA_ADMIN/admin/clients" \
  -H "Content-Type: application/json" \
  -d "{
    \"client_id\": \"$CLIENT_ID\",
    \"client_secret\": \"$CLIENT_SECRET\",
    \"grant_types\": [\"authorization_code\", \"refresh_token\"],
    \"response_types\": [\"code\"],
    \"scope\": \"openid profile email\",
    \"redirect_uris\": [\"http://localhost:8080/callback\"]
  }")

if echo "$CLIENT_RESULT" | jq -e '.client_id' > /dev/null 2>&1; then
  echo -e "    ${GREEN}OAuth2 client created!${NC}"
  echo "    Client ID: $CLIENT_ID"
else
  echo -e "    ${YELLOW}Client creation result:${NC}"
  echo "$CLIENT_RESULT" | jq .
fi

echo ""

# ==========================================
# 5. OAuth2 Authorization Flow
# ==========================================
echo "=== 5. OAuth2 Authorization Flow ==="
echo ""

echo "5.1 Start authorization..."
AUTH_URL="$HYDRA_PUBLIC/oauth2/auth?client_id=$CLIENT_ID&response_type=code&scope=openid+profile+email&redirect_uri=http://localhost:8080/callback&state=e2e-test-state"

AUTH_RESPONSE=$(curl -s -w "\n%{redirect_url}" -o /dev/null "$AUTH_URL")
echo "    Auth URL initiated"
echo "    Redirect: ${AUTH_RESPONSE:0:80}..."

echo ""
echo -e "    ${YELLOW}Note: Full OAuth2 flow requires browser interaction${NC}"
echo "    The authorization URL would redirect to Kratos login,"
echo "    then to consent, and finally return with an auth code."

echo ""

# ==========================================
# 6. List Identities (Admin)
# ==========================================
echo "=== 6. Admin: List Identities ==="
echo ""

IDENTITIES=$(curl -s "$KRATOS_PUBLIC/../admin/identities" 2>/dev/null || curl -s "http://localhost:4434/admin/identities")
IDENTITY_COUNT=$(echo "$IDENTITIES" | jq 'length')
echo "    Total identities: $IDENTITY_COUNT"
echo "$IDENTITIES" | jq -r '.[] | "    - \(.traits.email) (ID: \(.id[0:8])...)"' 2>/dev/null || echo "    (Could not list identities)"

echo ""

# ==========================================
# 7. Logout Flow
# ==========================================
echo "=== 7. Logout Flow ==="
echo ""

echo "7.1 Perform logout..."
LOGOUT_RESULT=$(curl -s -X DELETE "$KRATOS_PUBLIC/self-service/logout/api" \
  -H "Authorization: Bearer $SESSION_TOKEN")

echo "7.2 Verify session invalidated..."
WHOAMI_AFTER=$(curl -s -X GET "$KRATOS_PUBLIC/sessions/whoami" \
  -H "Authorization: Bearer $SESSION_TOKEN")

if echo "$WHOAMI_AFTER" | jq -e '.error' > /dev/null 2>&1; then
  echo -e "    ${GREEN}Logout successful - session invalidated${NC}"
else
  echo -e "    ${YELLOW}Session may still be valid${NC}"
fi

echo ""
echo "=========================================="
echo -e "  ${GREEN}E2E Test Completed${NC}"
echo "=========================================="
