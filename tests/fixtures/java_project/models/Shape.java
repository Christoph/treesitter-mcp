package com.example.calculator.models;

/**
 * Interface for geometric shapes
 */
public interface Shape {
    /**
     * Calculates the area of the shape
     */
    double area();
    
    /**
     * Calculates the perimeter of the shape
     */
    double perimeter();
    
    /**
     * Gets the center point of the shape
     */
    Point getCenter();
}

/**
 * Represents a circle
 */
class Circle implements Shape {
    private Point center;
    private double radius;
    
    /**
     * Creates a new circle
     */
    public Circle(Point center, double radius) {
        if (radius < 0) {
            throw new IllegalArgumentException("Radius must be non-negative");
        }
        this.center = center;
        this.radius = radius;
    }
    
    /**
     * Gets the center of the circle
     */
    @Override
    public Point getCenter() {
        return center;
    }
    
    /**
     * Gets the radius
     */
    public double getRadius() {
        return radius;
    }
    
    /**
     * Sets the radius
     */
    public void setRadius(double radius) {
        if (radius < 0) {
            throw new IllegalArgumentException("Radius must be non-negative");
        }
        this.radius = radius;
    }
    
    /**
     * Calculates the area of the circle
     */
    @Override
    public double area() {
        return Math.PI * radius * radius;
    }
    
    /**
     * Calculates the perimeter (circumference) of the circle
     */
    @Override
    public double perimeter() {
        return 2 * Math.PI * radius;
    }
    
    @Override
    public String toString() {
        return "Circle(center: " + center + ", radius: " + radius + ")";
    }
}

/**
 * Represents a rectangle
 */
class Rectangle implements Shape {
    private Point center;
    private double width;
    private double height;
    
    /**
     * Creates a new rectangle
     */
    public Rectangle(Point center, double width, double height) {
        this.center = center;
        this.width = width;
        this.height = height;
    }
    
    @Override
    public Point getCenter() {
        return center;
    }
    
    public double getWidth() {
        return width;
    }
    
    public double getHeight() {
        return height;
    }
    
    @Override
    public double area() {
        return width * height;
    }
    
    @Override
    public double perimeter() {
        return 2 * (width + height);
    }
}
