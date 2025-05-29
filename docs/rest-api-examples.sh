#!/bin/bash
# Ratchet REST API Examples
# These curl commands demonstrate how to interact with the Ratchet REST API

# Set the base URL (adjust for your environment)
BASE_URL="http://localhost:8080/api/v1"

echo "=== Ratchet REST API Examples ==="
echo "Base URL: $BASE_URL"
echo ""

# Tasks Examples
echo "--- TASKS ---"

echo "1. List all tasks with pagination:"
curl -X GET "$BASE_URL/tasks?_start=0&_end=10" \
  -H "Accept: application/json" | jq

echo -e "\n2. Get a specific task:"
# Replace with actual task UUID
curl -X GET "$BASE_URL/tasks/550e8400-e29b-41d4-a716-446655440000" \
  -H "Accept: application/json" | jq

echo -e "\n3. List tasks with filtering:"
curl -X GET "$BASE_URL/tasks?label_like=weather" \
  -H "Accept: application/json" | jq

echo -e "\n4. List tasks with sorting:"
curl -X GET "$BASE_URL/tasks?_sort=updated_at&_order=DESC" \
  -H "Accept: application/json" | jq

# Executions Examples
echo -e "\n--- EXECUTIONS ---"

echo "5. List all executions:"
curl -X GET "$BASE_URL/executions?_start=0&_end=10" \
  -H "Accept: application/json" | jq

echo -e "\n6. Create a new execution:"
curl -X POST "$BASE_URL/executions" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{
    "task_id": "1",
    "input": {
      "city": "New York",
      "units": "imperial"
    }
  }' | jq

echo -e "\n7. Get execution details:"
# Replace with actual execution ID
curl -X GET "$BASE_URL/executions/1" \
  -H "Accept: application/json" | jq

echo -e "\n8. List executions with status filter:"
curl -X GET "$BASE_URL/executions?status=completed&_start=0&_end=10" \
  -H "Accept: application/json" | jq

echo -e "\n9. Cancel a running execution:"
# Replace with actual execution ID
curl -X POST "$BASE_URL/executions/1/cancel" \
  -H "Accept: application/json" | jq

echo -e "\n10. Retry a failed execution:"
# Replace with actual execution ID
curl -X POST "$BASE_URL/executions/1/retry" \
  -H "Accept: application/json" | jq

echo -e "\n11. Update execution (mark as completed):"
# Replace with actual execution ID
curl -X PATCH "$BASE_URL/executions/1" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{
    "status": "completed",
    "output": {
      "temperature": 72,
      "humidity": 65,
      "conditions": "Partly cloudy"
    }
  }' | jq

echo -e "\n12. Delete an execution:"
# Replace with actual execution ID
curl -X DELETE "$BASE_URL/executions/1" \
  -H "Accept: application/json"

# Advanced filtering examples
echo -e "\n--- ADVANCED FILTERING ---"

echo "13. Filter executions by date range:"
curl -X GET "$BASE_URL/executions?queued_after=2024-01-01T00:00:00Z&queued_before=2024-12-31T23:59:59Z" \
  -H "Accept: application/json" | jq

echo -e "\n14. Filter executions by multiple statuses:"
curl -X GET "$BASE_URL/executions?status_in=failed,cancelled" \
  -H "Accept: application/json" | jq

echo -e "\n15. Check response headers (pagination info):"
curl -I -X GET "$BASE_URL/tasks?_start=0&_end=10" \
  -H "Accept: application/json"

echo -e "\n=== End of Examples ==="