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
 */
export function subtract(a: number, b: number): number {
    return a - b;
}

/**
 * Multiplies two numbers
 */
export function multiply(a: number, b: number): number {
    return a * b;
}

/**
 * Divides a by b, returns null if b is zero
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
 */
export function applyOperation(a: number, b: number, operation: Operation): number {
    const result = operation(a, b);
    
    // Nested arrow function
    const formatter = (x: number): string => `Result is: ${x}`;
    
    console.log(formatter(result));
    return result;
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
     */
    add(n: number): number {
        this.value += n;
        this._history.push(`add ${n}`);
        return this.value;
    }
    
    /**
     * Subtracts a number from the current value
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
