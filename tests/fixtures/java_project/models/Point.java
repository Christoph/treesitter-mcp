package com.example.calculator.models;

/**
 * Represents a point in 2D space
 */
public class Point {
    private double x;
    private double y;
    
    /**
     * Creates a new point at the origin
     */
    public Point() {
        this.x = 0;
        this.y = 0;
    }
    
    /**
     * Creates a point with specified coordinates
     */
    public Point(double x, double y) {
        this.x = x;
        this.y = y;
    }
    
    /**
     * Gets the X coordinate
     */
    public double getX() {
        return x;
    }
    
    /**
     * Sets the X coordinate
     */
    public void setX(double x) {
        this.x = x;
    }
    
    /**
     * Gets the Y coordinate
     */
    public double getY() {
        return y;
    }
    
    /**
     * Sets the Y coordinate
     */
    public void setY(double y) {
        this.y = y;
    }
    
    /**
     * Creates a point at the origin
     */
    public static Point origin() {
        return new Point(0, 0);
    }
    
    /**
     * Calculates distance from the origin
     */
    public double distanceFromOrigin() {
        return Math.sqrt(x * x + y * y);
    }
    
    /**
     * Calculates distance to another point
     */
    public double distanceTo(Point other) {
        double dx = x - other.x;
        double dy = y - other.y;
        return Math.sqrt(dx * dx + dy * dy);
    }
    
    /**
     * Translates the point by the given offset
     */
    public void translate(double dx, double dy) {
        this.x += dx;
        this.y += dy;
    }
    
    /**
     * Returns a new point translated by the given offset
     */
    public Point translated(double dx, double dy) {
        return new Point(x + dx, y + dy);
    }
    
    @Override
    public String toString() {
        return "Point(" + x + ", " + y + ")";
    }
}
