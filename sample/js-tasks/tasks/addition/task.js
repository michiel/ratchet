// Simple addition task for testing
function main(inputs) {
    const { a, b } = inputs;
    
    // Validate inputs
    if (typeof a !== 'number' || typeof b !== 'number') {
        throw new Error('Both inputs must be numbers');
    }
    
    const result = a + b;
    
    return { result };
}

module.exports = { main };