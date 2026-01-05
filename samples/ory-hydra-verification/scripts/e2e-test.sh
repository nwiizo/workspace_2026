#!/bin/bash
set -e

# E2E Test Script for Ory Hydra Authentication Verification
# Usage: ./scripts/e2e-test.sh

HYDRA_PUBLIC="http://localhost:4444"
HYDRA_ADMIN="http://localhost:4445"
LOGIN_PROVIDER="http://localhost:3000"
COOKIE_JAR="/tmp/oauth_e2e_cookies.txt"

echo "================================"
echo "E2E Test: Ory Hydra OAuth2 Flow"
echo "================================"
echo ""

# Cleanup
rm -f "$COOKIE_JAR"

# 1. Health Check
echo "1. Health Check..."
echo "   Login Provider: $(curl -s ${LOGIN_PROVIDER}/health | jq -r '.status')"
echo "   Hydra: $(curl -s ${HYDRA_PUBLIC}/health/ready | jq -r '.status')"
echo ""

# 2. Create OAuth2 Client
echo "2. Creating OAuth2 Client..."
# Delete client if exists
curl -s -X DELETE "${HYDRA_ADMIN}/admin/clients/e2e-test-client" > /dev/null 2>&1 || true

CLIENT_RESPONSE=$(curl -s -X POST "${HYDRA_ADMIN}/admin/clients" \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "e2e-test-client",
    "client_secret": "e2e-test-secret",
    "grant_types": ["authorization_code", "refresh_token"],
    "response_types": ["code"],
    "scope": "openid profile email",
    "redirect_uris": ["http://localhost:8080/callback"],
    "token_endpoint_auth_method": "client_secret_post"
  }')

CLIENT_ID=$(echo "$CLIENT_RESPONSE" | jq -r '.client_id')
echo "   Client created: $CLIENT_ID"
echo ""

# 3. Start OAuth2 Authorization Flow
echo "3. Starting OAuth2 Authorization Flow..."
STATE=$(openssl rand -hex 16)
AUTH_URL="${HYDRA_PUBLIC}/oauth2/auth?client_id=e2e-test-client&response_type=code&scope=openid+profile+email&redirect_uri=http://localhost:8080/callback&state=${STATE}"

# Get login challenge from redirect (don't follow redirects)
LOGIN_REDIRECT=$(curl -s -c "$COOKIE_JAR" -b "$COOKIE_JAR" -w "%{redirect_url}" -o /dev/null "$AUTH_URL")
LOGIN_CHALLENGE=$(echo "$LOGIN_REDIRECT" | sed -n 's/.*login_challenge=\([^&]*\).*/\1/p')
echo "   Login Challenge: ${LOGIN_CHALLENGE:0:30}..."
echo ""

# 4. Submit Login Form
echo "4. Submitting Login Form..."
# Submit login and get the Hydra redirect URL
LOGIN_RESPONSE=$(curl -s -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -w "\n%{redirect_url}" \
  -X POST "${LOGIN_PROVIDER}/login" \
  -d "login_challenge=${LOGIN_CHALLENGE}&email=demo@example.com&password=password123")

HYDRA_REDIRECT=$(echo "$LOGIN_RESPONSE" | tail -1)
echo "   Hydra Redirect: ${HYDRA_REDIRECT:0:80}..."

# Follow redirect to Hydra which will redirect to consent
CONSENT_REDIRECT=$(curl -s -c "$COOKIE_JAR" -b "$COOKIE_JAR" -w "%{redirect_url}" -o /dev/null "$HYDRA_REDIRECT")
CONSENT_CHALLENGE=$(echo "$CONSENT_REDIRECT" | sed -n 's/.*consent_challenge=\([^&]*\).*/\1/p')
echo "   Consent Challenge: ${CONSENT_CHALLENGE:0:30}..."
echo ""

# 5. Submit Consent Form
echo "5. Submitting Consent Form..."
# Submit consent and get the Hydra redirect URL
CONSENT_RESPONSE=$(curl -s -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -w "\n%{redirect_url}" \
  -X POST "${LOGIN_PROVIDER}/consent" \
  -d "consent_challenge=${CONSENT_CHALLENGE}")

CONSENT_HYDRA_REDIRECT=$(echo "$CONSENT_RESPONSE" | tail -1)
echo "   Consent Hydra Redirect: ${CONSENT_HYDRA_REDIRECT:0:80}..."

# Follow redirect to Hydra which will redirect to callback
CALLBACK_REDIRECT=$(curl -s -c "$COOKIE_JAR" -b "$COOKIE_JAR" -w "%{redirect_url}" -o /dev/null "$CONSENT_HYDRA_REDIRECT")
echo "   Callback Redirect: ${CALLBACK_REDIRECT:0:80}..."

# Extract authorization code
AUTH_CODE=$(echo "$CALLBACK_REDIRECT" | sed -n 's/.*code=\([^&]*\).*/\1/p')
RETURNED_STATE=$(echo "$CALLBACK_REDIRECT" | sed -n 's/.*state=\([^&]*\).*/\1/p')
echo "   Authorization Code: ${AUTH_CODE:0:40}..."
echo "   State Match: $([ "$STATE" = "$RETURNED_STATE" ] && echo "OK" || echo "MISMATCH (expected: $STATE, got: $RETURNED_STATE)")"
echo ""

# 6. Exchange Code for Tokens
echo "6. Exchanging Authorization Code for Tokens..."
TOKEN_RESPONSE=$(curl -s -X POST "${HYDRA_PUBLIC}/oauth2/token" \
  -d "grant_type=authorization_code" \
  -d "code=${AUTH_CODE}" \
  -d "redirect_uri=http://localhost:8080/callback" \
  -d "client_id=e2e-test-client" \
  -d "client_secret=e2e-test-secret")

ACCESS_TOKEN=$(echo "$TOKEN_RESPONSE" | jq -r '.access_token')
ID_TOKEN=$(echo "$TOKEN_RESPONSE" | jq -r '.id_token')

if [ "$ACCESS_TOKEN" = "null" ] || [ -z "$ACCESS_TOKEN" ]; then
  echo "   ERROR: Failed to get tokens"
  echo "   Response: $TOKEN_RESPONSE"
  echo ""
  echo "   Debug Info:"
  echo "   - Auth Code: $AUTH_CODE"
  echo "   - State: $STATE"
  echo "   - Returned State: $RETURNED_STATE"
  exit 1
fi

echo "   Access Token: ${ACCESS_TOKEN:0:40}..."
echo "   ID Token: ${ID_TOKEN:0:40}..."
echo ""

# 7. Decode and verify ID Token
echo "7. Decoding ID Token..."
# JWT base64url needs padding fixed
JWT_PAYLOAD=$(echo "$ID_TOKEN" | cut -d'.' -f2)
# Add padding if needed
case $((${#JWT_PAYLOAD} % 4)) in
  2) JWT_PAYLOAD="${JWT_PAYLOAD}==" ;;
  3) JWT_PAYLOAD="${JWT_PAYLOAD}=" ;;
esac
# Replace URL-safe characters
JWT_PAYLOAD=$(echo "$JWT_PAYLOAD" | tr '_-' '/+')
# Decode (macOS uses -D, Linux uses -d)
ID_TOKEN_PAYLOAD=$(echo "$JWT_PAYLOAD" | (base64 -d 2>/dev/null || base64 -D 2>/dev/null))
echo "   ID Token Payload:"
echo "$ID_TOKEN_PAYLOAD" | jq '.'
echo ""

# 8. Verify claims
echo "8. Verifying Claims..."
SUBJECT=$(echo "$ID_TOKEN_PAYLOAD" | jq -r '.sub')
EMAIL=$(echo "$ID_TOKEN_PAYLOAD" | jq -r '.email')
ROLE=$(echo "$ID_TOKEN_PAYLOAD" | jq -r '.role')
echo "   Subject: $SUBJECT"
echo "   Email: $EMAIL"
echo "   Role: $ROLE"
echo ""

# Validation
if [ -z "$SUBJECT" ] || [ "$SUBJECT" = "null" ]; then
  echo "   ERROR: Subject is missing"
  exit 1
fi

if [ "$EMAIL" = "demo@example.com" ]; then
  echo "   Email verification: PASSED"
else
  echo "   Email verification: FAILED (expected demo@example.com, got $EMAIL)"
fi

if [ "$ROLE" = "customer" ]; then
  echo "   Role verification: PASSED"
else
  echo "   Role verification: FAILED (expected customer, got $ROLE)"
fi
echo ""

# 9. Cleanup
echo "9. Cleanup..."
curl -s -X DELETE "${HYDRA_ADMIN}/admin/clients/e2e-test-client" > /dev/null
rm -f "$COOKIE_JAR"
echo "   Done"
echo ""

echo "================================"
echo "E2E Test Completed Successfully!"
echo "================================"
