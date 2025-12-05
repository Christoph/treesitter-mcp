/**
 * Calculator module with basic arithmetic operations
 */

import { Point, CalculatorOptions } from './types/models';

/**
 * Adds two numbers together
 * @param a - First number
 * @param b - Second number
 * @returns The sum
 */
export function add(a: number, b: number): number {
    return a + b;
}

/**
 * Subtracts b from a
 * @param a - The minuend
 * @param b - The subtrahend
 * @returns The difference
 */
export function subtract(a: number, b: number): number {
    return a - b;
}

/**
 * Multiplies two numbers
 * @param a - First number
 * @param b - Second number
 * @returns The product
 */
export function multiply(a: number, b: number): number {
    return a * b;
}

/**
 * Divides a by b, returns null if b is zero
 * @param a - The dividend
 * @param b - The divisor
 * @returns The quotient or null
 */
export function divide(a: number, b: number): number | null {
    if (b === 0) {
        return null;
    }
    return a / b;
}

/**
 * Private helper function
 */
function _privateHelper(x: number): boolean {
    return x >= 0;
}

/**
 * Type for operation functions
 */
type Operation = (a: number, b: number) => number;

/**
 * Applies a custom operation to two numbers
 * @param a - First number
 * @param b - Second number
 * @param operation - Operation function
 * @returns Result of the operation
 */
export function applyOperation(a: number, b: number, operation: Operation): number {
    const result = operation(a, b);
    
    // Nested arrow function
    const formatter = (x: number): string => `Result is: ${x}`;
    
    console.log(formatter(result));
    return result;
}

/**
 * A complex operation with nested functions
 * @param base - The base value
 * @returns The result
 */
export function complexOperation(base: number): number {
    const multiplier = 2;
    
    // First nested function
    const double = (x: number): number => x * multiplier;
    
    // Nested function inside the first
    const applyTwice = (x: number): number => {
        const firstPass = double(x);
        
        // Another nested function
        const addBase = (y: number): number => y + base;
        
        return addBase(firstPass);
    };
    
    return applyTwice(base);
}

/**
 * Calculates the distance between two points
 * @param p1 - First point
 * @param p2 - Second point
 * @returns The distance
 */
export function pointDistance(p1: Point, p2: Point): number {
    const dx = p1.x - p2.x;
    const dy = p1.y - p2.y;
    return Math.sqrt(dx * dx + dy * dy);
}

/**
 * A simple calculator class
 * 
 * This class maintains state for calculator operations.
 */
export class Calculator {
    private _history: string[];
    public value: number;
    
    /**
     * Creates a new Calculator
     * @param initialValue - Starting value (default: 0)
     */
    constructor(initialValue: number = 0) {
        this.value = initialValue;
        this._history = [];
    }
    
    /**
     * Adds a number to the current value
     * @param n - Number to add
     * @returns The new value
     */
    add(n: number): number {
        this.value += n;
        this._history.push(`add ${n}`);
        return this.value;
    }
    
    /**
     * Subtracts a number from the current value
     * @param n - Number to subtract
     * @returns The new value
     */
    subtract(n: number): number {
        this.value -= n;
        this._history.push(`subtract ${n}`);
        return this.value;
    }
    
    /**
     * Resets the calculator to zero
     */
    reset(): void {
        this.value = 0;
        this._history = [];
    }
    
    /**
     * Gets the operation history
     * @returns Copy of the history
     */
    getHistory(): string[] {
        return [...this._history];
    }
    
    /**
     * Private helper method
     */
    private _logOperation(op: string): void {
        this._history.push(op);
    }
    
    toString(): string {
        return `Calculator(value: ${this.value})`;
    }
}
