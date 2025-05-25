# Weather API Task Tests

This directory contains test cases for the Weather API task. Tests are defined as JSON files with specific fields that describe the input, expected output, and mock data for API responses.

## Task Implementation

The Weather API task is implemented as a pure JavaScript function that:

1. Takes city and units as input parameters
2. Uses the fetch API to call the OpenWeatherMap service
3. Returns weather data in a structured format

The implementation has no awareness of testing or mocks. It makes pure API calls through the fetch() function, which gets intercepted by the test framework during testing.

## Test Structure and HTTP Mocking

Each test file follows this structure:

```json
{
  "input": {
    // Input parameters for the task
  },
  "expected_output": {
    // Expected output from the task
  },
  "mock": {
    // Mock HTTP data for testing
    "http": {
      "url": "api.openweathermap.org",
      "method": "GET",
      "response": {
        "ok": true,
        "status": 200,
        "statusText": "OK",
        "body": {
          // The API response body
        }
      }
    }
  }
}
```

The `mock` section provides data that the test framework should return when the task makes fetch() calls. This allows testing the task without making real API requests.

## Current Testing Status

**Note:** Currently, the tests are failing with a "Schema validation error: 'success' is a required property" message. This suggests that the mocking system in Ratchet isn't properly intercepting the fetch() calls or returning the expected mock data.

The issue could be:
1. The mock system may not be fully implemented yet
2. There might be a specific way to structure mock data that we're not following
3. The task might need a special configuration to work with mocks

## Example Tests

1. `test-001.json` - Test for London with metric units
2. `test-002.json` - Test for New York with imperial units
3. `test-003-standard.json` - Test for London with imperial units
4. `test-003-with-mock.json` - Test for Berlin with metric units
5. `test-004-mock-api.json` - Test for Paris with metric units
6. `test-005-api-failure.json` - Test for a non-existent city (error case)

## Running Tests

Tests can be run using the `ratchet test` command:

```bash
ratchet test --from-fs sample/js-tasks/weather-api
```

## Real Production Implementation

The current implementation:

1. Uses a placeholder API key ("your-api-key-here")
2. Makes standard fetch() calls to the OpenWeatherMap API
3. Handles success and error responses appropriately
4. Has no awareness of testing or mocks

In a production environment, you would:

1. Replace the placeholder API key with a real one
2. Consider adding more error handling
3. Potentially add caching for frequently requested cities

## Dual Approach for Development

For development purposes only, you might consider maintaining two versions of the code:

1. **Production Version (current)**: Uses pure fetch() calls, no test awareness
2. **Development Version**: Hard-codes responses for common test cases

This allows faster development cycles without API calls, while ensuring the production version remains pure.