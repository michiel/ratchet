function main(input, context) {
    const { endpoint = "/json", include_headers = false } = input;
    
    // Construct the full URL
    const baseUrl = "https://httpbin.org";
    const fullUrl = baseUrl + endpoint;
    
    try {
        // Prepare request headers
        const headers = {
            'User-Agent': 'Ratchet-Test-Fetch/1.0'
        };
        
        if (include_headers) {
            headers['X-Test-Header'] = 'Ratchet-Sample-Task';
            headers['Accept'] = 'application/json';
        }
        
        // Make HTTP request using the fetch API
        const response = fetch(fullUrl, {
            method: 'GET',
            headers: headers
        });
        
        // Extract response information
        const requestInfo = {
            method: 'GET',
            url: fullUrl,
            headers_sent: headers,
            headers_included: include_headers
        };
        
        const fetchMetadata = {
            endpoint_requested: endpoint,
            timestamp: new Date().toISOString(),
            execution_context: context ? context.executionId || 'unknown' : 'no-context'
        };
        
        // Validate that we got a real response
        if (!response || typeof response !== 'object') {
            throw new Error("Invalid response from fetch API");
        }
        
        // Check if the request was successful
        if (!response.ok) {
            throw new NetworkError(`HTTP ${response.status}: ${response.statusText || 'Request failed'}`);
        }
        
        return {
            success: true,
            status: response.status,
            url: fullUrl,
            data: {
                response_body: response.body,
                request_info: requestInfo,
                fetch_metadata: fetchMetadata
            }
        };
        
    } catch (error) {
        // Handle different types of errors appropriately
        let errorType = 'UnknownError';
        if (error instanceof NetworkError) {
            errorType = 'NetworkError';
        } else if (error instanceof DataError) {
            errorType = 'DataError';
        }
        
        return {
            success: false,
            status: 0,
            url: fullUrl,
            data: {
                response_body: null,
                request_info: {
                    method: 'GET',
                    url: fullUrl,
                    headers_sent: include_headers ? headers : {'User-Agent': 'Ratchet-Test-Fetch/1.0'},
                    headers_included: include_headers,
                    error: error.message,
                    error_type: errorType
                },
                fetch_metadata: {
                    endpoint_requested: endpoint,
                    timestamp: new Date().toISOString(),
                    execution_context: context ? context.executionId || 'unknown' : 'no-context',
                    failed: true
                }
            }
        };
    }
}