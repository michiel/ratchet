# Test Fetch Sample Task

This sample task demonstrates how to use the fetch API in Ratchet to retrieve data from the internet over HTTP.

## Description

The `test-fetch` task makes HTTP requests to httpbin.org endpoints to demonstrate real network connectivity and data retrieval capabilities.

## Features

- **Real HTTP requests**: Makes actual network calls to httpbin.org
- **Multiple endpoints**: Supports different httpbin.org test endpoints
- **Custom headers**: Option to include custom HTTP headers
- **Error handling**: Proper error handling with typed error responses
- **Response validation**: Validates that real data was retrieved

## Usage

### Basic JSON fetch
```bash
ratchet run-once --from-fs sample/js-tasks/tasks/test-fetch --input-json='{"endpoint": "/json"}'
```

### Fetch with custom headers
```bash
ratchet run-once --from-fs sample/js-tasks/tasks/test-fetch --input-json='{"endpoint": "/get", "include_headers": true}'
```

### Fetch UUID
```bash
ratchet run-once --from-fs sample/js-tasks/tasks/test-fetch --input-json='{"endpoint": "/uuid"}'
```

### Fetch IP address
```bash
ratchet run-once --from-fs sample/js-tasks/tasks/test-fetch --input-json='{"endpoint": "/ip"}'
```

## Input Parameters

- `endpoint` (required): The httpbin.org endpoint to fetch from
  - `/json` - Returns sample JSON slideshow data
  - `/get` - Returns request information including headers
  - `/uuid` - Returns a generated UUID
  - `/ip` - Returns the requester's IP address
- `include_headers` (optional): Whether to include custom headers in the request

## Output

Returns an object containing:
- `success`: Boolean indicating if the request was successful
- `status`: HTTP status code
- `url`: The complete URL that was fetched
- `data`: Object containing:
  - `response_body`: The actual response from httpbin.org
  - `request_info`: Information about the request that was made
  - `fetch_metadata`: Metadata about the fetch operation

## Testing

Run the test suite:
```bash
ratchet test --from-fs sample/js-tasks/tasks/test-fetch
```

This task includes three test cases:
1. Basic JSON endpoint fetch
2. GET request with custom headers
3. UUID generation endpoint

## Purpose

This sample demonstrates:
- How to use the fetch API for HTTP requests
- Proper error handling in network operations
- Working with different types of HTTP responses
- Including custom headers in requests
- Validating that real network data was retrieved

The task serves as both a functional example and a test to verify that the Ratchet fetch implementation can successfully retrieve data from the internet over HTTP.