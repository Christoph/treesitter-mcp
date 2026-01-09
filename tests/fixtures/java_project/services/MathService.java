package com.example.calculator.services;

import com.example.calculator.models.Point;
import java.util.List;
import java.util.stream.Collectors;

/**
 * Service class for advanced mathematical operations
 */
public class MathService {
    
    private static final double EPSILON = 1e-10;
    
    /**
     * Calculates factorial of a number
     */
    public static long factorial(int n) {
        if (n < 0) {
            throw new IllegalArgumentException("Factorial is not defined for negative numbers");
        }
        if (n <= 1) {
            return 1;
        }
        return n * factorial(n - 1);
    }
    
    /**
     * Calculates the nth Fibonacci number
     */
    public static int fibonacci(int n) {
        if (n <= 1) {
            return n;
        }
        return fibonacci(n - 1) + fibonacci(n - 2);
    }
    
    /**
     * Checks if a number is prime
     */
    public static boolean isPrime(int n) {
        if (n <= 1) {
            return false;
        }
        if (n <= 3) {
            return true;
        }
        if (n % 2 == 0 || n % 3 == 0) {
            return false;
        }
        for (int i = 5; i * i <= n; i += 6) {
            if (n % i == 0 || n % (i + 2) == 0) {
                return false;
            }
        }
        return true;
    }
    
    /**
     * Calculates the distance between two points
     */
    public static double calculateDistance(Point p1, Point p2) {
        return p1.distanceTo(p2);
    }
    
    /**
     * Finds the centroid of a list of points
     */
    public static Point calculateCentroid(List<Point> points) {
        if (points.isEmpty()) {
            return Point.origin();
        }
        
        double sumX = points.stream().mapToDouble(Point::getX).sum();
        double sumY = points.stream().mapToDouble(Point::getY).sum();
        
        return new Point(sumX / points.size(), sumY / points.size());
    }
    
    /**
     * Compares two doubles for equality within epsilon
     */
    public static boolean doubleEquals(double a, double b) {
        return Math.abs(a - b) < EPSILON;
    }
}
