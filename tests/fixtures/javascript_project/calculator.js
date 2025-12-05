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
 * @param {number} a - The minuend
 * @param {number} b - The subtrahend
 * @returns {number} The difference
 */
function subtract(a, b) {
    return a - b;
}

/**
 * Multiplies two numbers
 * @param {number} a - First number
 * @param {number} b - Second number
 * @returns {number} The product
 */
function multiply(a, b) {
    return a * b;
}

/**
 * Divides a by b, returns null if b is zero
 * @param {number} a - The dividend
 * @param {number} b - The divisor
 * @returns {number|null} The quotient or null
 */
function divide(a, b) {
    if (b === 0) {
        return null;
    }
    return a / b;
}

function applyOperation(a, b, operation) {
    
    
    
    
    const formatter = (x) => `Result is: ${x}`;
    const result = operation(a, b);
    
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
    
    
    add(n) {
        this.value += n;
        this._history.push(`add ${n}`);
        return this.value;
    }
    
    /**
     * Subtracts a number from the current value
     * @param {number} n - Number to subtract
     * @returns {number} The new value
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
     * @returns {Array<string>} Copy of the history
     */
    getHistory() {
        return [...this._history];
    }
    
    /**
     * Private helper method
     * @private
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
    /**
     * Creates a new point
     * @param {number} x - X coordinate
     * @param {number} y - Y coordinate
     */
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    
    /**
     * Creates a point at the origin
     * @static
     * @returns {Point} A point at (0, 0)
     */
    static origin() {
        return new Point(0, 0);
    }
    
    /**
     * Calculates distance from origin
     * @returns {number} The distance
     */
    distanceFromOrigin() {
        return Math.sqrt(this.x ** 2 + this.y ** 2);
    }
    
    /**
     * Calculates distance to another point
     * @param {Point} other - The other point
     * @returns {number} The distance
     */
    distanceTo(other) {
        const dx = this.x - other.x;
        const dy = this.y - other.y;
        return Math.sqrt(dx ** 2 + dy ** 2);
    }
    
    /**
     * Translates the point by the given offset
     * @param {number} dx - X offset
     * @param {number} dy - Y offset
     */
    translate(dx, dy) {
        this.x += dx;
        this.y += dy;
    }
    
    /**
     * Returns a new point translated by the given offset
     * @param {number} dx - X offset
     * @param {number} dy - Y offset
     * @returns {Point} A new translated point
     */
    translated(dx, dy) {
        return new Point(this.x + dx, this.y + dy);
    }
}

/**
 * A line segment between two points
 */
class LineSegment {
    /**
     * Creates a new line segment
     * @param {Point} start - Starting point
     * @param {Point} end - Ending point
     */
    constructor(start, end) {
        this.start = start;
        this.end = end;
    }
    
    /**
     * Calculates the length of the line segment
     * @returns {number} The length
     */
    length() {
        return this.start.distanceTo(this.end);
    }
    
    /**
     * Calculates the midpoint of the line segment
     * @returns {Point} The midpoint
     */
    midpoint() {
        const midX = (this.start.x + this.end.x) / 2;
        const midY = (this.start.y + this.end.y) / 2;
        return new Point(midX, midY);
    }
}

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
    LineSegment
};
