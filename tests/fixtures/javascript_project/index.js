/**
 * JavaScript Project Test Fixture
 * 
 * This is a test fixture for testing tree-sitter parsing of JavaScript code.
 */

const { add, subtract, multiply, Calculator } = require('./calculator');
const { validateInput, formatResult } = require('./utils/helpers');

// Re-export main functions
module.exports = {
    add,
    subtract,
    multiply,
    Calculator,
    validateInput,
    formatResult
};

/**
 * A helper function at the module level
 */
function processResult(value) {
    return formatResult(value);
}

module.exports.processResult = processResult;
