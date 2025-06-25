
function multiply(input) {
    const { a, b } = input;
    if (typeof a !== 'number' || typeof b !== 'number') {
        throw new Error('Both a and b must be numbers');
    }
    return { result: a * b };
}

// Export for Ratchet
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { multiply };
}