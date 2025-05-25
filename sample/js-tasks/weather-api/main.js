(function(input) {
    // Extract parameters from input
    const city = input.city || "Unknown";
    const units = input.units || "metric";
    
    // Weather API key - in a real implementation, this would be securely managed
    const API_KEY = "your-api-key-here";
    
    // Build the API URL
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
})
