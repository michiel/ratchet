# Weather API Task

This task demonstrates how to retrieve weather data for a given city.

## Description

The Weather API task takes a city name and optional units parameter and returns current weather information including temperature, description, and humidity.

## Implementation Notes

The current implementation uses hard-coded responses for demonstration and testing purposes. In a real-world scenario, you would replace this with actual API calls to a weather service like OpenWeatherMap.

### Real-World Implementation

The commented section in `main.js` shows how to implement this task with real API calls:

```javascript
const url = `https://api.openweathermap.org/data/2.5/weather?q=${encodeURIComponent(city)}&units=${units}&appid=${API_KEY}`;

try {
    // Make the HTTP request using the fetch API
    const response = fetch(url, { method: "GET" });
    
    // Check if the request was successful
    if (!response.ok) {
        throw new Error(`API error: ${response.status} ${response.statusText}`);
    }
    
    // Parse the response body
    const data = response.body;
    
    // Format and return the weather data
    return {
        location: `${data.name}, ${data.sys.country}`,
        temperature: data.main.temp,
        units: units === "metric" ? "C" : "F",
        description: data.weather[0].description,
        humidity: data.main.humidity
    };
} catch (error) {
    throw new Error(`Failed to fetch weather data: ${error.message}`);
}
```

To use this implementation, you would:
1. Uncomment the code
2. Remove the hard-coded responses
3. Replace "your-api-key-here" with an actual API key

## Input Schema

The task accepts:
- `city` (required): The name of the city to get weather for
- `units` (optional): Temperature units, either "metric" (Celsius) or "imperial" (Fahrenheit)

## Output Schema

The task returns:
- `location`: City and country code (e.g., "London, GB")
- `temperature`: Current temperature
- `units`: "C" for Celsius or "F" for Fahrenheit
- `description`: Weather description (e.g., "partly cloudy")
- `humidity`: Humidity percentage

## Testing

The `tests` directory contains test cases for this task. See the [README.md](tests/README.md) in that directory for more information about the testing approach.