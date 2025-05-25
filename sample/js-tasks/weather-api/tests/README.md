# Weather API Task Tests

This directory contains test cases for the Weather API task. Tests are defined as JSON files with specific fields that describe the input, expected output, and optional mock data for API responses.

## Test Structure

Each test file should follow this structure:

```json
{
  "input": {
    // Input parameters that match the task's input schema
  },
  "expected_output": {
    // Expected output that matches the task's output schema
  },
  "mock": {
    // Optional mock data for HTTP requests
    "http": {
      "url": "example.com",      // URL substring to match
      "method": "GET",           // HTTP method to match
      "response": {
        "ok": true,              // Whether the response was successful
        "status": 200,           // HTTP status code
        "statusText": "OK",      // HTTP status text
        "body": {
          // The response body to return
        }
      }
    }
  }
}
```

## Example Tests

1. `test-001.json` - Basic test for London with metric units
2. `test-002.json` - Basic test for New York with imperial units
3. `test-003-standard.json` - Test for London with imperial units
4. `test-004-mock-api.json` - Test with mock HTTP response for Paris
5. `test-005-api-failure.json` - Test with mock HTTP failure response

## Running Tests

Tests can be run using the `ratchet test` command:

```bash
ratchet test --from-fs sample/js-tasks/weather-api
```

## Testing Approach

The Weather API task is designed to work both online and offline:

1. For common cities (London, New York, Berlin, Tokyo, Sydney), it returns hardcoded responses
2. For other cities, it would normally make API calls to OpenWeatherMap
3. During testing, the mock system intercepts HTTP requests and returns predefined responses
4. This allows for comprehensive testing without requiring an internet connection or API keys

## Mock HTTP Responses

The mock system allows you to:

1. Match requests by URL and method
2. Provide custom responses with status codes and bodies
3. Test error handling by simulating API failures
4. Simulate different data scenarios

This approach ensures that tests are reliable, repeatable, and don't depend on external services.