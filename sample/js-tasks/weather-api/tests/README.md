# Weather API Task Tests

This directory contains test cases for the Weather API task. Tests are defined as JSON files with specific fields that describe the input, expected output, and mock data for API responses.

## Task Implementation

The Weather API task is implemented as a simple JavaScript function that:

1. Takes city and units as input parameters
2. Would typically use the fetch API to call a weather service
3. Returns weather data in a structured format

For demonstration purposes, the implementation returns hard-coded values based on the city name. This simulates the behavior of a real API call without requiring an actual API key or internet connection.

**Note:** The file includes commented-out code showing how the fetch API would be used in a real implementation. In practice, you would remove the hard-coded values and uncomment the fetch code.

## Test Structure

Each test file follows this structure:

```json
{
  "input": {
    // Input parameters that match the task's input schema
  },
  "expected_output": {
    // Expected output that matches the task's output schema
  },
  "mock": {
    // Documentation of what API responses would look like
    "http": {
      "url": "api.openweathermap.org",
      "method": "GET",
      "response": {
        "body": {
          // The API response body that would be returned
        }
      }
    }
  }
}
```

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

## API Response Documentation

While not actively used in the current implementation, the mock section in each test file documents:

1. What URL would be called
2. What HTTP method would be used
3. What response body would be expected from the real API

This serves as documentation for developers who want to understand how the API would behave if implemented with real HTTP calls.

## Benefits of This Approach

1. **Simplified Testing**: Tests don't rely on complex mocking mechanisms
2. **No External Dependencies**: No need for API keys or internet access
3. **Deterministic Results**: Tests always return the same output for the same input
4. **API Documentation**: Test files document expected API responses
5. **Easy Upgrades**: Can be updated to use real fetch calls by uncommenting the code

This approach provides a balance between realism (through documented API responses) and simplicity (through deterministic behavior).