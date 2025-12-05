/**
 * Helper utilities for the calculator
 */

/**
 * Validates that input is non-negative
 * @param {number} x - Number to validate
 * @returns {boolean} True if valid
 */
function validateInput(x) {
    return x >= 0;
}

/**
 * Formats a result value
 * @param {number} value - The value to format
 * @returns {string} Formatted string
 */
function formatResult(value) {
    return `Result: ${value}`;
}

/**
 * Clamps a value between min and max
 * @param {number} value - The value to clamp
 * @param {number} minVal - Minimum value
 * @param {number} maxVal - Maximum value
 * @returns {number} The clamped value
 */
function clamp(value, minVal, maxVal) {
    return Math.max(minVal, Math.min(value, maxVal));
}

/**
 * Applies a function to all values in an array
 * @param {Array} values - List of values
 * @param {Function} func - Function to apply
 * @returns {Array} List of results
 */
function applyToAll(values, func) {
    return values.map(func);
}

/**
 * Composes two functions
 * @param {Function} f - First function
 * @param {Function} g - Second function
 * @returns {Function} A new function that applies g then f
 */
function compose(f, g) {
    return (x) => f(g(x));
}

module.exports = {
    validateInput,
    formatResult,
    clamp,
    applyToAll,
    compose
};
