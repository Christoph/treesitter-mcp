/**
 * JavaScript Project Test Fixture
 * 
 * This is a test fixture for testing tree-sitter parsing of JavaScript code.
 */

const {
    add,
    subtract,
    multiply,
    divide,
    applyOperation,
    complexOperation,
    pointDistance,
    Calculator,
    Point,
    LineSegment
} = require('./calculator');

const { validateInput, formatResult, clamp } = require('./utils/helpers');

// Re-export main functions
module.exports = {
    add,
    subtract,
    multiply,
    divide,
    applyOperation,
    complexOperation,
    pointDistance,
    Calculator,
    Point,
    LineSegment,
    validateInput,
    formatResult,
    clamp,
    processResult,
    createCalculator,
    performSequence
};

/**
 * A helper function at the module level
 * @param {number} value - The value to process
 * @returns {string} Formatted result
 */
function processResult(value) {
    return formatResult(value);
}

/**
 * Creates a calculator with an initial value
 * @param {number} initialValue - Starting value
 * @returns {Calculator} A new calculator instance
 */
function createCalculator(initialValue = 0) {
    return new Calculator(initialValue);
}

/**
 * Performs a sequence of operations on a calculator
 * @param {Calculator} calc - The calculator
 * @param {Array<Array>} operations - Array of [value, operator] pairs
 * @returns {number} The final value
 */
function performSequence(calc, operations) {
    for (const [value, op] of operations) {
        switch (op) {
            case '+':
                calc.add(value);
                break;
            case '-':
                calc.subtract(value);
                break;
            default:
                break;
        }
    }
    return calc.value;
}
