/**
 * TypeScript Project Test Fixture
 * 
 * This is a test fixture for testing tree-sitter parsing of TypeScript code.
 */

import {
    add,
    subtract,
    multiply,
    divide,
    applyOperation,
    complexOperation,
    pointDistance,
    Calculator
} from './calculator';

import { Point, CalculatorOptions, PointImpl, Result, Ok, Err } from './types/models';

// Re-export main functions
export { add, subtract, multiply, divide, applyOperation, complexOperation, pointDistance, Calculator };
export type { Point, CalculatorOptions, Result };
export { PointImpl, Ok, Err };

/**
 * A helper function at the module level
 * @param value - The value to format
 * @returns Formatted result
 */
export function formatResult(value: number): string {
    return `Result: ${value}`;
}

/**
 * Creates a calculator with options
 * @param options - Calculator options
 * @returns A new calculator instance
 */
export function createCalculator(options?: CalculatorOptions): Calculator {
    return new Calculator(options?.initialValue);
}

/**
 * Performs a sequence of operations on a calculator
 * @param calc - The calculator
 * @param operations - Array of [value, operator] pairs
 * @returns The final value
 */
export function performSequence(
    calc: Calculator,
    operations: Array<[number, '+' | '-']>
): number {
    for (const [value, op] of operations) {
        switch (op) {
            case '+':
                calc.add(value);
                break;
            case '-':
                calc.subtract(value);
                break;
        }
    }
    return calc.value;
}

/**
 * Creates a result wrapper
 * @param value - The value to wrap
 * @returns A result object
 */
export function wrapResult<T>(value: T): Result<T> {
    return Ok(value);
}

/**
 * Unwraps a result or throws an error
 * @param result - The result to unwrap
 * @returns The value if ok
 * @throws Error if result is an error
 */
export function unwrapResult<T>(result: Result<T>): T {
    if (result.ok) {
        return result.value;
    }
    throw result.error;
}
