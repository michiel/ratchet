# Running the Schedule + Webhook Integration Tests

The updated `tests/rest_api_workflow_e2e_test.rs` now includes comprehensive webhook integration tests that cover your requested scenario:

## Available Tests

### 1. Core Scenario Test (Recommended)
**Test**: `test_schedule_webhook_integration_core_scenario`

This test specifically implements your requested scenario:
1. ✅ **Adds a schedule via API**
2. ✅ **Monitors for scheduled job execution**  
3. ✅ **Verifies webhook delivery of return values**

```bash
# Run the core scenario test
cargo test test_schedule_webhook_integration_core_scenario --test rest_api_workflow_e2e_test -- --nocapture

# Or run with environment variables for faster execution
RATCHET_FAST_TESTS=1 cargo test test_schedule_webhook_integration_core_scenario --test rest_api_workflow_e2e_test -- --nocapture
```

### 2. Complete Workflow Test
**Test**: `test_complete_schedule_workflow_with_webhook`

This test includes additional features:
- Task creation if none exist
- Manual schedule triggering
- Detailed webhook payload validation
- Comprehensive cleanup

```bash
# Run the complete workflow test
cargo test test_complete_schedule_workflow_with_webhook --test rest_api_workflow_e2e_test -- --nocapture
```

## Test Features

### What These Tests Do

1. **Start Real Server**: Full ratchet server with database and scheduler
2. **Create Webhook Server**: Test webhook endpoint to capture HTTP POST calls
3. **Sample Task Setup**: Addition task that calculates `a + b = result`
4. **Schedule Creation**: Schedule with webhook output destination configured
5. **Job Monitoring**: Wait for scheduled jobs to be created and executed
6. **Webhook Verification**: Verify HTTP POST is sent to webhook with execution results
7. **Result Validation**: Verify webhook payload contains correct execution output

### Key Test Scenarios

#### Schedule → Job → Execution → Webhook Flow
```
📅 Schedule Created (every minute)
     ↓
💼 Job Created (by scheduler)
     ↓  
⚡ Job Executed (by job processor)
     ↓
📨 Webhook HTTP POST (with results)
```

#### Manual Trigger Test
```
⚡ POST /schedules/{id}/trigger
     ↓
💼 Immediate Job Creation
     ↓
⚡ Job Execution
     ↓ 
📨 Webhook HTTP POST (with results)
```

### Expected Output

When working correctly, you should see:
```
🎯 Testing core scenario: Schedule → Job → Execution → Webhook
✅ Using task: addition (task-123)
✅ Created schedule: schedule-456
✅ Found 1 job(s) created by schedule
✅ Found 1 execution(s)
  ⚡ Execution 0: status=COMPLETED
    📤 Output: {"result": 8}
🎉 SUCCESS: Received 1 webhook payload(s)!
📨 Webhook payload: {
  "task_id": "task-123",
  "status": "completed", 
  "output": {"result": 8},
  "timestamp": "2025-06-20T..."
}
🎯 Core scenario SUCCESSFUL: Schedule → Job → Execution → Webhook ✅
```

## Current Implementation Status

Based on our previous work, the tests will:

✅ **Server Setup**: Full server starts correctly  
✅ **Schedule Creation**: API endpoints working  
✅ **Job Creation**: Scheduler creates jobs from schedules  
✅ **Job Processing**: JobProcessorService processes queued jobs  
✅ **Execution Creation**: DirectExecutionRepository creates executions  
❓ **Webhook Delivery**: Depends on output delivery implementation

The webhook integration will work once the output delivery system is connected to the execution completion in the JobProcessorService.

## Running Other Tests

```bash
# Run all REST API workflow tests
cargo test --test rest_api_workflow_e2e_test -- --nocapture

# Run specific test patterns
cargo test schedule.*webhook --test rest_api_workflow_e2e_test -- --nocapture

# Run with faster timeouts
RATCHET_FAST_TESTS=1 cargo test --test rest_api_workflow_e2e_test -- --nocapture
```

The tests are designed to be robust and informative, showing exactly which parts of the pipeline are working and which need further implementation.