#!/bin/bash
# API Security Demo - Vulnerability Test Script
#
# OWASP API Security Top 10 vulnerability demonstrations
# Each demo shows vulnerable vs secure endpoint comparison
#
# Usage:
#   ./scripts/test_all.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PASS_COUNT=0
FAIL_COUNT=0

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

log_info() {
    echo -e "${YELLOW}[INFO]${NC} $1"
}

log_header() {
    echo -e "${BLUE}$1${NC}"
}

wait_for_server() {
    local max_attempts=30
    local attempt=1
    while ! curl -s http://localhost:8080/health > /dev/null 2>&1 && \
          ! curl -s http://localhost:8080/ > /dev/null 2>&1 && \
          ! curl -s http://localhost:8080/token/alice > /dev/null 2>&1; do
        if [ $attempt -ge $max_attempts ]; then
            echo "Server failed to start"
            return 1
        fi
        sleep 0.2
        attempt=$((attempt + 1))
    done
    return 0
}

cleanup() {
    pkill -f "target/release/" 2>/dev/null || true
    sleep 0.5
}

# HTTP response helpers
get_response() {
    local response
    response=$(curl -s -w "|||%{http_code}" "$@")
    echo "$response"
}

get_body() {
    echo "$1" | sed 's/|||[0-9]*$//'
}

get_code() {
    echo "$1" | grep -o '|||[0-9]*$' | sed 's/|||//'
}

echo "=========================================="
echo "API Security Demo - Vulnerability Tests"
echo "OWASP API Security Top 10"
echo "=========================================="
echo ""

# Build
log_info "Building all demos..."
cargo build --release --quiet

trap cleanup EXIT

# ===========================================
# BOLA: Broken Object Level Authorization
# ===========================================
echo ""
log_header "=========================================="
log_header "BOLA (Broken Object Level Authorization)"
log_header "=========================================="
cleanup
cargo run --release --bin bola-demo &>/dev/null &
sleep 2
wait_for_server

BOB_TOKEN=$(curl -s http://localhost:8080/token/bob | jq -r .access_token)

echo ""
echo "Attack: Bob tries to access Alice's order (order_id=1)"
echo ""

VULN_RESPONSE=$(get_response -H "Authorization: Bearer $BOB_TOKEN" http://localhost:8080/vulnerable/orders/1)
VULN_BODY=$(get_body "$VULN_RESPONSE")
VULN_CODE=$(get_code "$VULN_RESPONSE")

if [ "$VULN_CODE" == "200" ] && echo "$VULN_BODY" | grep -q '"user":"alice"'; then
    log_pass "Vulnerable EP: Bob accessed Alice's order (HTTP $VULN_CODE)"
else
    log_fail "Vulnerable EP: Attack should succeed (HTTP $VULN_CODE)"
fi

SECURE_RESPONSE=$(get_response -H "Authorization: Bearer $BOB_TOKEN" http://localhost:8080/orders/1)
SECURE_BODY=$(get_body "$SECURE_RESPONSE")
SECURE_CODE=$(get_code "$SECURE_RESPONSE")

if [ "$SECURE_CODE" == "404" ]; then
    log_pass "Secure EP: Access denied (HTTP $SECURE_CODE)"
else
    log_fail "Secure EP: Attack should be blocked (HTTP $SECURE_CODE)"
fi

cleanup

# ===========================================
# BFLA: Broken Function Level Authorization
# ===========================================
echo ""
log_header "=========================================="
log_header "BFLA (Broken Function Level Authorization)"
log_header "=========================================="
cargo run --release --bin bfla-demo &>/dev/null &
sleep 2
wait_for_server

USER_TOKEN=$(curl -s http://localhost:8080/token/user | jq -r .access_token)

echo ""
echo "Attack: Regular user tries to access admin functions"
echo ""

VULN_RESPONSE=$(get_response -H "Authorization: Bearer $USER_TOKEN" http://localhost:8080/vulnerable/admin/users)
VULN_BODY=$(get_body "$VULN_RESPONSE")
VULN_CODE=$(get_code "$VULN_RESPONSE")

if [ "$VULN_CODE" == "200" ] && echo "$VULN_BODY" | grep -q '"ssn"'; then
    log_pass "Vulnerable EP: Regular user accessed admin data (HTTP $VULN_CODE)"
else
    log_fail "Vulnerable EP: Attack should succeed (HTTP $VULN_CODE)"
fi

SECURE_RESPONSE=$(get_response -H "Authorization: Bearer $USER_TOKEN" http://localhost:8080/admin/users)
SECURE_BODY=$(get_body "$SECURE_RESPONSE")
SECURE_CODE=$(get_code "$SECURE_RESPONSE")

if [ "$SECURE_CODE" == "403" ]; then
    log_pass "Secure EP: Forbidden (HTTP $SECURE_CODE)"
else
    log_fail "Secure EP: Attack should be blocked (HTTP $SECURE_CODE)"
fi

cleanup

# ===========================================
# Mass Assignment
# ===========================================
echo ""
log_header "=========================================="
log_header "Mass Assignment Attack"
log_header "=========================================="
cargo run --release --bin mass-assignment-demo &>/dev/null &
sleep 2
wait_for_server

ALICE_TOKEN=$(curl -s http://localhost:8080/token/alice | jq -r .access_token)

echo ""
echo "Attack: Manipulate payment status to 'approved'"
echo ""

VULN_RESPONSE=$(get_response -X POST http://localhost:8080/vulnerable/payments \
    -H "Authorization: Bearer $ALICE_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"amount": 100, "currency": "USD", "status": "approved"}')
VULN_BODY=$(get_body "$VULN_RESPONSE")
VULN_CODE=$(get_code "$VULN_RESPONSE")

if [ "$VULN_CODE" == "200" ] && echo "$VULN_BODY" | grep -q '"status":"approved"'; then
    log_pass "Vulnerable EP: Payment created with status=approved (HTTP $VULN_CODE)"
else
    log_fail "Vulnerable EP: Attack should succeed (HTTP $VULN_CODE)"
fi

SECURE_RESPONSE=$(get_response -X POST http://localhost:8080/payments \
    -H "Authorization: Bearer $ALICE_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"amount": 100, "currency": "USD", "status": "approved"}')
SECURE_BODY=$(get_body "$SECURE_RESPONSE")
SECURE_CODE=$(get_code "$SECURE_RESPONSE")

if [ "$SECURE_CODE" == "200" ] && echo "$SECURE_BODY" | grep -q '"status":"pending"'; then
    log_pass "Secure EP: Status ignored, created as pending (HTTP $SECURE_CODE)"
else
    log_fail "Secure EP: Status should be pending (HTTP $SECURE_CODE)"
fi

cleanup

# ===========================================
# Broken Authentication
# ===========================================
echo ""
log_header "=========================================="
log_header "Broken Authentication (JWT Validation)"
log_header "=========================================="
cargo run --release --bin broken-auth-demo &>/dev/null &
sleep 2
wait_for_server

EXPIRED_TOKEN=$(curl -s http://localhost:8080/token/expired | jq -r .access_token)

echo ""
echo "Attack: Access with expired token"
echo ""

VULN_RESPONSE=$(get_response -H "Authorization: Bearer $EXPIRED_TOKEN" http://localhost:8080/vulnerable/validate)
VULN_BODY=$(get_body "$VULN_RESPONSE")
VULN_CODE=$(get_code "$VULN_RESPONSE")

if [ "$VULN_CODE" == "200" ] && echo "$VULN_BODY" | grep -q '"valid":true'; then
    log_pass "Vulnerable EP: Expired token accepted (HTTP $VULN_CODE)"
else
    log_fail "Vulnerable EP: Attack should succeed (HTTP $VULN_CODE)"
fi

SECURE_RESPONSE=$(get_response -H "Authorization: Bearer $EXPIRED_TOKEN" http://localhost:8080/validate)
SECURE_BODY=$(get_body "$SECURE_RESPONSE")
SECURE_CODE=$(get_code "$SECURE_RESPONSE")

if [ "$SECURE_CODE" == "401" ]; then
    log_pass "Secure EP: Expired token rejected (HTTP $SECURE_CODE)"
else
    log_fail "Secure EP: Attack should be blocked (HTTP $SECURE_CODE)"
fi

cleanup

# ===========================================
# Rate Limiting (Brute Force Protection)
# ===========================================
echo ""
log_header "=========================================="
log_header "Rate Limiting (Brute Force Protection)"
log_header "=========================================="
cargo run --release --bin rate-limit-demo &>/dev/null &
sleep 2
wait_for_server

echo ""
echo "Attack: 5 consecutive login failures (account lock)"
echo ""

# 5 failed attempts
for i in {1..5}; do
    curl -s -X POST http://localhost:8080/login \
        -H "Content-Type: application/json" \
        -d '{"email":"user@example.com","password":"wrong"}' > /dev/null
done

# 6th attempt
RESULT=$(curl -s -X POST http://localhost:8080/login \
    -H "Content-Type: application/json" \
    -d '{"email":"user@example.com","password":"wrong"}')

if echo "$RESULT" | grep -q "Account locked"; then
    log_pass "Account locked after 5 failures"
else
    log_fail "Account should be locked"
fi

cleanup

# ===========================================
# SSRF (Server-Side Request Forgery)
# ===========================================
echo ""
log_header "=========================================="
log_header "SSRF (Server-Side Request Forgery)"
log_header "=========================================="
cargo run --release --bin ssrf-demo &>/dev/null &
sleep 2
wait_for_server

echo ""
echo "Attack: Access internal secrets via SSRF"
echo ""

VULN_RESPONSE=$(get_response -X POST http://localhost:8080/vulnerable/fetch \
    -H "Content-Type: application/json" \
    -d '{"url":"http://localhost:8080/internal/secrets"}')
VULN_BODY=$(get_body "$VULN_RESPONSE")
VULN_CODE=$(get_code "$VULN_RESPONSE")

if [ "$VULN_CODE" == "200" ] && echo "$VULN_BODY" | grep -q "database_password"; then
    log_pass "Vulnerable EP: Internal secrets exposed (HTTP $VULN_CODE)"
else
    log_fail "Vulnerable EP: Attack should succeed (HTTP $VULN_CODE)"
fi

SECURE_RESPONSE=$(get_response -X POST http://localhost:8080/fetch \
    -H "Content-Type: application/json" \
    -d '{"url":"http://localhost:8080/internal/secrets"}')
SECURE_BODY=$(get_body "$SECURE_RESPONSE")
SECURE_CODE=$(get_code "$SECURE_RESPONSE")

if [ "$SECURE_CODE" == "400" ] && echo "$SECURE_BODY" | grep -q "Only HTTPS"; then
    log_pass "Secure EP: HTTP URL blocked (HTTP $SECURE_CODE)"
else
    log_fail "Secure EP: Attack should be blocked (HTTP $SECURE_CODE)"
fi

cleanup

# ===========================================
# JWT Token Handling
# ===========================================
echo ""
log_header "=========================================="
log_header "JWT Token Generation & Validation"
log_header "=========================================="
cargo run --release --bin jwt-demo &>/dev/null &
sleep 2
wait_for_server

echo ""
echo "Test: HS256/RS256 token generation and validation"
echo ""

# HS256
HS256_TOKEN=$(curl -s http://localhost:8080/token/hs256 | jq -r .access_token)
HS256_VALID=$(curl -s -H "Authorization: Bearer $HS256_TOKEN" http://localhost:8080/validate/hs256 | jq -r .valid)

if [ "$HS256_VALID" == "true" ]; then
    log_pass "HS256: Token generated and validated"
else
    log_fail "HS256: Token validation failed"
fi

# RS256
RS256_TOKEN=$(curl -s http://localhost:8080/token/rs256 | jq -r .access_token)
RS256_VALID=$(curl -s -H "Authorization: Bearer $RS256_TOKEN" http://localhost:8080/validate/rs256 | jq -r .valid)

if [ "$RS256_VALID" == "true" ]; then
    log_pass "RS256: Token generated and validated"
else
    log_fail "RS256: Token validation failed"
fi

# Invalid token
INVALID_RESULT=$(curl -s -H "Authorization: Bearer invalid.token.here" http://localhost:8080/validate/hs256)
if echo "$INVALID_RESULT" | grep -q '"valid":false'; then
    log_pass "Invalid token: Correctly rejected"
else
    log_fail "Invalid token: Should be rejected"
fi

cleanup

# ===========================================
# Security Observability
# ===========================================
echo ""
log_header "=========================================="
log_header "Security Observability (Monitoring)"
log_header "=========================================="
cargo run --release --bin observability-demo &>/dev/null &
sleep 2
wait_for_server

echo ""
echo "Test: Suspicious request detection"
echo ""

# SQLi pattern
curl -s "http://localhost:8080/api/data?id=1%27%20OR%201=1--" > /dev/null

# Auth failure
curl -s -H "Authorization: Bearer invalid" http://localhost:8080/api/protected > /dev/null

# Check metrics
METRICS=$(curl -s http://localhost:8080/metrics)

if echo "$METRICS" | grep -q '"sql_injection_attempts":1'; then
    log_pass "SQLi detection: Recorded in metrics"
else
    log_fail "SQLi detection: Should be recorded"
fi

if echo "$METRICS" | grep -q '"failed_auth_attempts":1'; then
    log_pass "Auth failure detection: Recorded in metrics"
else
    log_fail "Auth failure detection: Should be recorded"
fi

cleanup

# ===========================================
# Security Testing
# ===========================================
echo ""
log_header "=========================================="
log_header "Security Testing (Data Exposure)"
log_header "=========================================="
cargo run --release --bin security-test-demo &>/dev/null &
sleep 2
wait_for_server

echo ""
echo "Test: Sensitive data exposure and input validation"
echo ""

# Sensitive data exposure
VULN_USERS=$(curl -s http://localhost:8080/vulnerable/users)
SECURE_USERS=$(curl -s http://localhost:8080/api/users)

if echo "$VULN_USERS" | grep -q '"ssn"'; then
    log_pass "Vulnerable EP: SSN exposed"
else
    log_fail "Vulnerable EP: SSN should be exposed"
fi

if ! echo "$SECURE_USERS" | grep -q '"ssn"'; then
    log_pass "Secure EP: SSN not exposed"
else
    log_fail "Secure EP: SSN should not be exposed"
fi

# SQL Injection input validation
SQLI_RESULT=$(curl -s "http://localhost:8080/api/search?q=%27%20OR%201=1--")

if echo "$SQLI_RESULT" | grep -q "Invalid characters"; then
    log_pass "SQLi input validation: Invalid characters blocked"
else
    log_fail "SQLi input validation: Should be blocked"
fi

# Built-in tests
TEST_RESULTS=$(curl -s http://localhost:8080/test/run-all)
PASSED=$(echo "$TEST_RESULTS" | jq -r .passed)
TOTAL=$(echo "$TEST_RESULTS" | jq -r .total)

if [ "$PASSED" == "$TOTAL" ]; then
    log_pass "Built-in tests: $PASSED/$TOTAL passed"
else
    log_fail "Built-in tests: $PASSED/$TOTAL passed"
fi

cleanup

# ===========================================
# Summary
# ===========================================
echo ""
log_header "=========================================="
log_header "Test Results Summary"
log_header "=========================================="
echo -e "${GREEN}PASS: $PASS_COUNT${NC}"
echo -e "${RED}FAIL: $FAIL_COUNT${NC}"
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}All security tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
