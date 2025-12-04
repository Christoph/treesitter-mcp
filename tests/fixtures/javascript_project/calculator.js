/**
 * Calculator module with basic arithmetic operations
 */

const { validateInput } = require('./utils/helpers');

/**
 * Adds two numbers together
 * @param {number} a - First number
 * @param {number} b - Second number
 * @returns {number} The sum
 */
function add(a, b) {
    return a + b;
}

/**
 * Subtracts b from a
 */
function subtract(a, b) {
    return a - b;
}

/**
 * Multiplies two numbers
 */
function multiply(a, b) {
    return a * b;
}

/**
 * Divides a by b, returns null if b is zero
 */
function divide(a, b) {
    if (b === 0) {
        return null;
    }
    return a / b;
}

/**
 * Private helper function
 */
function _privateHelper(x) {
    return x >= 0;
}

/**
 * Applies a custom operation to two numbers
 */
function applyOperation(a, b, operation) {
    const result = operation(a, b);
    
    // Nested arrow function
    const formatter = (x) => `Result is: ${x}`;
    
    console.log(formatter(result));
    return result;
}

/**
 * A simple calculator class
 * 
 * This class maintains state for calculator operations.
 */
class Calculator {
    /**
     * Creates a new Calculator
     * @param {number} initialValue - Starting value (default: 0)
     */
    constructor(initialValue = 0) {
        this.value = initialValue;
        this._history = [];
    }
    
    /**
     * Adds a number to the current value
     */
    add(n) {
        this.value += n;
        this._history.push(`add ${n}`);
        return this.value;
    }
    
    /**
     * Subtracts a number from the current value
     */
    subtract(n) {
        this.value -= n;
        this._history.push(`subtract ${n}`);
        return this.value;
    }
    
    /**
     * Resets the calculator to zero
     */
    reset() {
        this.value = 0;
        this._history = [];
    }
    
    /**
     * Gets the operation history
     */
    getHistory() {
        return [...this._history];
    }
    
    /**
     * Private helper method
     */
    _logOperation(op) {
        this._history.push(op);
    }
    
    toString() {
        return `Calculator(value: ${this.value})`;
    }
}

/**
 * A point class for testing
 */
class Point {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    
    /**
     * Calculates distance from origin
     */
    distanceFromOrigin() {
        return Math.sqrt(this.x ** 2 + this.y ** 2);
    }
}

module.exports = {
    add,
    subtract,
    multiply,
    divide,
    applyOperation,
    Calculator,
    Point
};
