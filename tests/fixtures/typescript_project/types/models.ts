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
    /**
     * Creates a new point
     * @param x - X coordinate
     * @param y - Y coordinate
     */
    constructor(public x: number, public y: number) {}
    
    /**
     * Creates a point at the origin
     * @static
     * @returns A point at (0, 0)
     */
    static origin(): PointImpl {
        return new PointImpl(0, 0);
    }
    
    /**
     * Calculates distance from origin
     * @returns The distance
     */
    distanceFromOrigin(): number {
        return Math.sqrt(this.x ** 2 + this.y ** 2);
    }
    
    /**
     * Calculates distance to another point
     * @param other - The other point
     * @returns The distance
     */
    distanceTo(other: Point): number {
        const dx = this.x - other.x;
        const dy = this.y - other.y;
        return Math.sqrt(dx ** 2 + dy ** 2);
    }
    
    /**
     * Translates the point by the given offset
     * @param dx - X offset
     * @param dy - Y offset
     */
    translate(dx: number, dy: number): void {
        this.x += dx;
        this.y += dy;
    }
    
    /**
     * Returns a new point translated by the given offset
     * @param dx - X offset
     * @param dy - Y offset
     * @returns A new translated point
     */
    translated(dx: number, dy: number): PointImpl {
        return new PointImpl(this.x + dx, this.y + dy);
    }
}

/**
 * A line segment between two points
 */
export class LineSegment {
    /**
     * Creates a new line segment
     * @param start - Starting point
     * @param end - Ending point
     */
    constructor(public start: Point, public end: Point) {}
    
    /**
     * Calculates the length of the line segment
     * @returns The length
     */
    length(): number {
        const dx = this.start.x - this.end.x;
        const dy = this.start.y - this.end.y;
        return Math.sqrt(dx ** 2 + dy ** 2);
    }
    
    /**
     * Calculates the midpoint of the line segment
     * @returns The midpoint
     */
    midpoint(): Point {
        return {
            x: (this.start.x + this.end.x) / 2,
            y: (this.start.y + this.end.y) / 2
        };
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
 * @param value - The value to wrap
 * @returns A success result
 */
export function Ok<T>(value: T): Result<T> {
    return { ok: true, value };
}

/**
 * Helper function to create error result
 * @param error - The error to wrap
 * @returns An error result
 */
export function Err<E = Error>(error: E): Result<never, E> {
    return { ok: false, error };
}

/**
 * A generic pair type
 */
export type Pair<T, U> = [T, U];

/**
 * A generic triple type
 */
export type Triple<T, U, V> = [T, U, V];

/**
 * A generic dictionary type
 */
export type Dictionary<T> = {
    [key: string]: T;
};

/**
 * A generic optional type
 */
export type Optional<T> = T | null | undefined;

/**
 * A generic async result type
 */
export type AsyncResult<T, E = Error> = Promise<Result<T, E>>;
