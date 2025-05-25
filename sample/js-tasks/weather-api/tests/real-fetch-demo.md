# Implementing Real HTTP Fetch Calls

This file documents how to update the Weather API task to use real fetch calls instead of hard-coded values.

## Step 1: Replace Hard-Coded Implementation

In `main.js`, uncomment the fetch implementation and remove the if/else statements:

```javascript
(function(input) {
    // Extract parameters from input
    const city = input.city || "Unknown";
    const units = input.units || "metric";
    
    // Weather API key (replace with your actual key)
    const API_KEY = "your-api-key-here";
    const url = `https://api.openweathermap.org/data/2.5/weather?q=${encodeURIComponent(city)}&units=${units}&appid=${API_KEY}`;
    
    try {
        // Make the HTTP request using the fetch API
        const response = fetch(url, { method: "GET" });
        
        // Check if the request was successful
        if (!response.ok) {
            return {
                success: false,
                error: `API error: ${response.status} ${response.statusText}`
            };
        }
        
        // Parse the response body
        const data = response.body;
        
        // Format and return the weather data
        return {
            success: true,
            location: `${data.name}, ${data.sys.country}`,
            temperature: data.main.temp,
            units: units === "metric" ? "C" : "F",
            description: data.weather[0].description,
            humidity: data.main.humidity
        };
    } catch (error) {
        // Handle any errors
        return {
            success: false,
            error: `Failed to fetch weather data: ${error.message}`
        };
    }
})
```

## Step 2: Test With Mocked HTTP Calls

When running tests with the Ratchet testing system:

1. The fetch call would be intercepted by the testing framework
2. Instead of making a real HTTP request, the mock data from the test files would be used
3. The function would receive the mock data and process it as if it came from the real API

Example mock response from test-001.json:

```json
{
  "name": "London",
  "sys": {
    "country": "GB"
  },
  "main": {
    "temp": 15.2,
    "humidity": 65
  },
  "weather": [
    {
      "description": "partly cloudy"
    }
  ]
}
```

## Step 3: Production Deployment

For production deployment:

1. Replace "your-api-key-here" with a real API key
2. Use environment variables or secure storage for the API key
3. Consider adding caching to reduce API calls
4. Add error handling for network timeouts and other failure modes

## Benefits of Using Real HTTP Calls

1. **Real-Time Data**: Get actual, current weather information
2. **Comprehensive Coverage**: Access data for any city in the world
3. **Additional Data**: Access more detailed weather information from the API
4. **Realistic Error Handling**: Test error conditions with real API responses

## Running Tests After Implementation

Tests will still work the same way after implementation, but they'll use the mock HTTP responses instead of making real API calls during test runs.