#!/bin/bash
set -e

OATHKEEPER_URL="http://localhost:4455"
KETO_WRITE_URL="http://localhost:4467"
KETO_READ_URL="http://localhost:4466"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass_count=0
fail_count=0

check() {
    local name="$1"
    local expected="$2"
    local actual="$3"

    if [ "$expected" = "$actual" ]; then
        echo -e "${GREEN}PASS${NC}: $name (expected=$expected, actual=$actual)"
        pass_count=$((pass_count + 1))
    else
        echo -e "${RED}FAIL${NC}: $name (expected=$expected, actual=$actual)"
        fail_count=$((fail_count + 1))
    fi
}

echo -e "${YELLOW}=== Waiting for services ===${NC}"
echo "Waiting for Oathkeeper..."
for i in {1..30}; do
    if curl -s http://localhost:4456/health/ready > /dev/null 2>&1; then
        echo "Oathkeeper is ready"
        break
    fi
    sleep 2
done

echo "Waiting for Keto..."
for i in {1..30}; do
    if curl -s http://localhost:4466/health/ready > /dev/null 2>&1; then
        echo "Keto is ready"
        break
    fi
    sleep 2
done

echo ""
echo -e "${YELLOW}=== Setting up Keto permissions ===${NC}"

# Create permissions in Keto
# alice is editor of doc1
curl -s -X PUT "$KETO_WRITE_URL/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Document",
    "object": "doc1",
    "relation": "editor",
    "subject_id": "alice"
  }' > /dev/null

# alice is also viewer of doc1 (editor implies viewer in real world, but we add explicitly here)
curl -s -X PUT "$KETO_WRITE_URL/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Document",
    "object": "doc1",
    "relation": "viewer",
    "subject_id": "alice"
  }' > /dev/null

# bob is only viewer of doc1
curl -s -X PUT "$KETO_WRITE_URL/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Document",
    "object": "doc1",
    "relation": "viewer",
    "subject_id": "bob"
  }' > /dev/null

# charlie has no permissions on doc1

echo "Permissions created:"
echo "  - alice: editor + viewer of doc1"
echo "  - bob: viewer of doc1"
echo "  - charlie: no permissions"

echo ""
echo -e "${YELLOW}=== Verifying Keto permissions ===${NC}"

# Verify permissions directly with Keto
alice_editor=$(curl -s -X POST "$KETO_READ_URL/relation-tuples/check" \
  -H "Content-Type: application/json" \
  -d '{"namespace": "Document", "object": "doc1", "relation": "editor", "subject_id": "alice"}' | jq -r '.allowed')
check "Keto: alice is editor of doc1" "true" "$alice_editor"

bob_viewer=$(curl -s -X POST "$KETO_READ_URL/relation-tuples/check" \
  -H "Content-Type: application/json" \
  -d '{"namespace": "Document", "object": "doc1", "relation": "viewer", "subject_id": "bob"}' | jq -r '.allowed')
check "Keto: bob is viewer of doc1" "true" "$bob_viewer"

bob_editor=$(curl -s -X POST "$KETO_READ_URL/relation-tuples/check" \
  -H "Content-Type: application/json" \
  -d '{"namespace": "Document", "object": "doc1", "relation": "editor", "subject_id": "bob"}' | jq -r '.allowed')
check "Keto: bob is NOT editor of doc1" "false" "$bob_editor"

charlie_viewer=$(curl -s -X POST "$KETO_READ_URL/relation-tuples/check" \
  -H "Content-Type: application/json" \
  -d '{"namespace": "Document", "object": "doc1", "relation": "viewer", "subject_id": "charlie"}' | jq -r '.allowed')
check "Keto: charlie is NOT viewer of doc1" "false" "$charlie_viewer"

echo ""
echo -e "${YELLOW}=== Testing Oathkeeper - Public Endpoints ===${NC}"

# Test public health endpoint
health_status=$(curl -s -o /dev/null -w "%{http_code}" "$OATHKEEPER_URL/health")
check "Public: /health returns 200" "200" "$health_status"

# Test public API endpoint
public_status=$(curl -s -o /dev/null -w "%{http_code}" "$OATHKEEPER_URL/api/public")
check "Public: /api/public returns 200" "200" "$public_status"

echo ""
echo -e "${YELLOW}=== Testing Oathkeeper - Protected Endpoints ===${NC}"

# Test protected endpoint (noop authenticator allows all, but header mutator passes X-User-Id)
protected_with_auth=$(curl -s -o /dev/null -w "%{http_code}" -H "X-User-Id: alice" "$OATHKEEPER_URL/api/protected")
check "Protected: /api/protected with auth returns 200" "200" "$protected_with_auth"

# Verify the header is passed through
protected_content=$(curl -s -H "X-User-Id: alice" "$OATHKEEPER_URL/api/protected")
if echo "$protected_content" | grep -q "alice"; then
    echo -e "${GREEN}PASS${NC}: Protected: X-User-Id header passed correctly"
    pass_count=$((pass_count + 1))
else
    echo -e "${RED}FAIL${NC}: Protected: X-User-Id header not passed"
    fail_count=$((fail_count + 1))
fi

echo ""
echo -e "${YELLOW}=== Testing Oathkeeper - Document View (GET) ===${NC}"

# alice can view doc1 (viewer permission)
alice_view=$(curl -s -o /dev/null -w "%{http_code}" -H "X-User-Id: alice" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: alice can GET doc1" "200" "$alice_view"

# bob can view doc1 (viewer permission)
bob_view=$(curl -s -o /dev/null -w "%{http_code}" -H "X-User-Id: bob" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: bob can GET doc1" "200" "$bob_view"

# charlie cannot view doc1 (no permission)
charlie_view=$(curl -s -o /dev/null -w "%{http_code}" -H "X-User-Id: charlie" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: charlie cannot GET doc1" "403" "$charlie_view"

# anonymous cannot view doc1 (Keto check fails with empty subject_id -> 403)
anon_view=$(curl -s -o /dev/null -w "%{http_code}" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: anonymous cannot GET doc1" "403" "$anon_view"

echo ""
echo -e "${YELLOW}=== Testing Oathkeeper - Document Edit (PUT) ===${NC}"

# alice can edit doc1 (editor permission)
alice_edit=$(curl -s -o /dev/null -w "%{http_code}" -X PUT -H "X-User-Id: alice" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: alice can PUT doc1" "200" "$alice_edit"

# bob cannot edit doc1 (only viewer, not editor)
bob_edit=$(curl -s -o /dev/null -w "%{http_code}" -X PUT -H "X-User-Id: bob" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: bob cannot PUT doc1" "403" "$bob_edit"

# charlie cannot edit doc1 (no permission)
charlie_edit=$(curl -s -o /dev/null -w "%{http_code}" -X PUT -H "X-User-Id: charlie" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: charlie cannot PUT doc1" "403" "$charlie_edit"

echo ""
echo -e "${YELLOW}=== Testing Oathkeeper - Document Delete (DELETE) ===${NC}"

# alice can delete doc1 (editor permission)
alice_delete=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE -H "X-User-Id: alice" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: alice can DELETE doc1" "200" "$alice_delete"

# bob cannot delete doc1 (only viewer)
bob_delete=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE -H "X-User-Id: bob" "$OATHKEEPER_URL/api/documents/doc1")
check "Document: bob cannot DELETE doc1" "403" "$bob_delete"

echo ""
echo -e "${YELLOW}=== Summary ===${NC}"
echo -e "Passed: ${GREEN}$pass_count${NC}"
echo -e "Failed: ${RED}$fail_count${NC}"

if [ $fail_count -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
