/**
 * TypeScript Project Test Fixture
 * 
 * This is a test fixture for testing tree-sitter parsing of TypeScript code.
 */

import { add, subtract, multiply, Calculator } from './calculator';
import { Point, CalculatorOptions } from './types/models';

// Re-export main functions
export { add, subtract, multiply, Calculator };
export type { Point, CalculatorOptions };

/**
 * A helper function at the module level
 */
export function formatResult(value: number): string {
    return `Result: ${value}`;
}

/**
 * Creates a calculator with options
 */
export function createCalculator(options?: CalculatorOptions): Calculator {
    return new Calculator(options?.initialValue);
}
