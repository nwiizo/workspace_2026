#!/bin/bash
set -e

echo "=========================================="
echo "  E2E Test: Ory Keto Authorization"
echo "=========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

KETO_READ="http://localhost:4466"
KETO_WRITE="http://localhost:4467"

# Wait for Keto to be ready
echo "Waiting for Keto..."
for i in {1..30}; do
    if curl -s "${KETO_READ}/health/ready" > /dev/null 2>&1; then
        echo -e "${GREEN}Keto is ready!${NC}"
        break
    fi
    echo "Waiting... ($i/30)"
    sleep 2
done

echo ""
echo "=== 1. Health Check ==="
HEALTH=$(curl -s "${KETO_READ}/health/ready")
echo "Keto: ${HEALTH}"

echo ""
echo "=== 2. Create Organization and Members ==="

# Create Organization:acme with members
echo "Creating Organization:acme..."

# Alice is admin and member of acme
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "admin",
    "subject_id": "alice"
  }' | jq .

curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "member",
    "subject_id": "alice"
  }' | jq .

# Bob is member of acme
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "member",
    "subject_id": "bob"
  }' | jq .

# Charlie is member of acme
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "member",
    "subject_id": "charlie"
  }' | jq .

echo -e "${GREEN}Organization members created${NC}"

echo ""
echo "=== 3. Create Project with Permissions ==="

# Project:alpha - alice is owner
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Project",
    "object": "alpha",
    "relation": "owner",
    "subject_id": "alice"
  }' | jq .

# Project:alpha - bob is editor
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Project",
    "object": "alpha",
    "relation": "editor",
    "subject_id": "bob"
  }' | jq .

# Project:alpha - all acme members are viewers (using subject set)
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Project",
    "object": "alpha",
    "relation": "viewer",
    "subject_set": {
      "namespace": "Organization",
      "object": "acme",
      "relation": "member"
    }
  }' | jq .

echo -e "${GREEN}Project permissions created${NC}"

echo ""
echo "=== 4. Create Document with Inherited Permissions ==="

# Document:doc1 - alice is editor
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Document",
    "object": "doc1",
    "relation": "editor",
    "subject_id": "alice"
  }' | jq .

# Document:doc1 - project editors are viewers
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Document",
    "object": "doc1",
    "relation": "viewer",
    "subject_set": {
      "namespace": "Project",
      "object": "alpha",
      "relation": "editor"
    }
  }' | jq .

# Document:secret - only alice can access
curl -s -X PUT "${KETO_WRITE}/admin/relation-tuples" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Document",
    "object": "secret",
    "relation": "viewer",
    "subject_id": "alice"
  }' | jq .

echo -e "${GREEN}Document permissions created${NC}"

echo ""
echo "=== 5. Permission Checks ==="
echo ""

check_permission() {
    local namespace=$1
    local object=$2
    local relation=$3
    local subject=$4
    local expected=$5

    result=$(curl -s -X POST "${KETO_READ}/relation-tuples/check" \
      -H "Content-Type: application/json" \
      -d "{
        \"namespace\": \"${namespace}\",
        \"object\": \"${object}\",
        \"relation\": \"${relation}\",
        \"subject_id\": \"${subject}\"
      }" | jq -r '.allowed')

    if [ "$result" == "$expected" ]; then
        echo -e "${GREEN}PASS${NC}: ${subject} ${relation} ${namespace}:${object} = ${result}"
    else
        echo -e "${RED}FAIL${NC}: ${subject} ${relation} ${namespace}:${object} = ${result} (expected: ${expected})"
    fi
}

echo "--- Organization Checks ---"
check_permission "Organization" "acme" "admin" "alice" "true"
check_permission "Organization" "acme" "admin" "bob" "false"
check_permission "Organization" "acme" "member" "alice" "true"
check_permission "Organization" "acme" "member" "bob" "true"
check_permission "Organization" "acme" "member" "charlie" "true"
check_permission "Organization" "acme" "member" "dave" "false"

echo ""
echo "--- Project Checks ---"
check_permission "Project" "alpha" "owner" "alice" "true"
check_permission "Project" "alpha" "owner" "bob" "false"
check_permission "Project" "alpha" "editor" "bob" "true"
check_permission "Project" "alpha" "editor" "charlie" "false"

echo ""
echo "--- Document Checks ---"
check_permission "Document" "doc1" "editor" "alice" "true"
check_permission "Document" "doc1" "editor" "bob" "false"
check_permission "Document" "secret" "viewer" "alice" "true"
check_permission "Document" "secret" "viewer" "bob" "false"

echo ""
echo "=== 6. List All Relation Tuples ==="
echo ""

echo "Organization tuples:"
curl -s "${KETO_READ}/relation-tuples?namespace=Organization" | jq '.relation_tuples[] | "\(.object)#\(.relation)@\(.subject_id // .subject_set.object)"'

echo ""
echo "Project tuples:"
curl -s "${KETO_READ}/relation-tuples?namespace=Project" | jq '.relation_tuples[] | "\(.object)#\(.relation)@\(.subject_id // (.subject_set.namespace + ":" + .subject_set.object + "#" + .subject_set.relation))"'

echo ""
echo "Document tuples:"
curl -s "${KETO_READ}/relation-tuples?namespace=Document" | jq '.relation_tuples[] | "\(.object)#\(.relation)@\(.subject_id // (.subject_set.namespace + ":" + .subject_set.object + "#" + .subject_set.relation))"'

echo ""
echo "=== 7. Expand API (Who can access?) ==="
echo ""

echo "Who can view Project:alpha?"
curl -s -X POST "${KETO_READ}/relation-tuples/expand" \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Project",
    "object": "alpha",
    "relation": "viewer",
    "max_depth": 3
  }' | jq '.tree'

echo ""
echo "=========================================="
echo -e "  ${GREEN}E2E Test Completed${NC}"
echo "=========================================="
