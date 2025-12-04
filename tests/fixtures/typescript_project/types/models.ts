/**
 * Type definitions and interfaces
 */

/**
 * A point interface for testing
 */
export interface Point {
    x: number;
    y: number;
}

/**
 * Calculator options interface
 */
export interface CalculatorOptions {
    initialValue?: number;
    maxHistory?: number;
}

/**
 * Operation result type
 */
export type OperationResult = {
    value: number;
    operation: string;
    timestamp: Date;
};

/**
 * A point class implementation
 */
export class PointImpl implements Point {
    constructor(public x: number, public y: number) {}
    
    /**
     * Calculates distance from origin
     */
    distanceFromOrigin(): number {
        return Math.sqrt(this.x ** 2 + this.y ** 2);
    }
    
    /**
     * Calculates distance to another point
     */
    distanceTo(other: Point): number {
        const dx = this.x - other.x;
        const dy = this.y - other.y;
        return Math.sqrt(dx ** 2 + dy ** 2);
    }
}

/**
 * Generic result type
 */
export type Result<T, E = Error> = 
    | { ok: true; value: T }
    | { ok: false; error: E };

/**
 * Helper function to create success result
 */
export function Ok<T>(value: T): Result<T> {
    return { ok: true, value };
}

/**
 * Helper function to create error result
 */
export function Err<E = Error>(error: E): Result<never, E> {
    return { ok: false, error };
}
