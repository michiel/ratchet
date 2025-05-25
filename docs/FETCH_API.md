# Fetch API in Ratchet

Ratchet includes a JavaScript `fetch` API that allows tasks to make HTTP requests to external services. This document explains how to use the API in your JavaScript tasks.

## Basic Usage

The fetch API follows a similar pattern to the browser's `fetch` function:

```javascript
// Make a GET request
const response = fetch(url, options);

// Check if the request was successful
if (response.ok) {
    // Access the response body
    const data = response.body;
    
    // Use the data
    console.log(data);
}
```

## API Reference

### fetch(url, options, body)

Makes an HTTP request to the specified URL.

#### Parameters

- **url** (string, required): The URL to send the request to.
- **options** (object, optional): Request configuration options.
  - **method** (string): HTTP method (GET, POST, PUT, DELETE, etc.). Defaults to "GET".
  - **headers** (object): HTTP headers to send with the request.
- **body** (object, optional): Request body for POST/PUT requests. Must be a JSON-serializable object.

#### Return Value

An object with the following properties:

- **ok** (boolean): Whether the request was successful (status in the range 200-299).
- **status** (number): The HTTP status code.
- **statusText** (string): The HTTP status text.
- **body** (object): The response body, parsed as JSON if possible.

## Examples

### Simple GET Request

```javascript
// Make a GET request to a JSON API
const response = fetch("https://api.example.com/data");

if (response.ok) {
    // Process the data
    const data = response.body;
    return {
        result: data.value
    };
} else {
    return {
        error: `Request failed with status ${response.status}`
    };
}
```

### POST Request with JSON Body

```javascript
// Make a POST request with a JSON body
const response = fetch(
    "https://api.example.com/create", 
    { method: "POST" },
    { name: "Example Item", value: 42 }
);

if (response.ok) {
    return {
        success: true,
        id: response.body.id
    };
} else {
    return {
        success: false,
        error: response.statusText
    };
}
```

### Request with Custom Headers

```javascript
// Make a request with custom headers
const response = fetch(
    "https://api.example.com/protected", 
    { 
        method: "GET",
        headers: {
            "Authorization": "Bearer YOUR_TOKEN",
            "Accept": "application/json"
        }
    }
);

// Process the response
if (response.ok) {
    return response.body;
} else {
    throw new Error(`API request failed: ${response.status}`);
}
```

## Limitations

- The fetch API in Ratchet is a simplified version of the browser's fetch API.
- Only JSON data is supported for request and response bodies.
- Streaming and binary data are not supported.
- There's no support for cookies or sessions.
- CORS restrictions don't apply since requests are made from the server.

## Example Task

Check the `sample/js-tasks/weather-api` directory for a complete example of using the fetch API to retrieve weather data from a public API.

## Error Handling

Always handle potential errors when using fetch:

```javascript
try {
    const response = fetch("https://api.example.com/data");
    
    if (!response.ok) {
        return {
            success: false,
            error: `API error: ${response.status} ${response.statusText}`
        };
    }
    
    return {
        success: true,
        data: response.body
    };
} catch (error) {
    return {
        success: false,
        error: `Network error: ${error.message}`
    };
}
```