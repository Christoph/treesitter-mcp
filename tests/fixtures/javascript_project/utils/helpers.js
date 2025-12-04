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
 */
function clamp(value, minVal, maxVal) {
    return Math.max(minVal, Math.min(value, maxVal));
}

module.exports = {
    validateInput,
    formatResult,
    clamp
};
