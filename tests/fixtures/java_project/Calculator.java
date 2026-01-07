package com.example.calculator;

import java.util.List;
import java.util.ArrayList;
import java.util.function.BiFunction;

/**
 * Calculator class with basic arithmetic operations
 */
public class Calculator {
    
    /**
     * Adds two integers
     * @param a first number
     * @param b second number
     * @return the sum
     */
    public static int add(int a, int b) {
        return a + b;
    }
    
    /**
     * Subtracts b from a
     * @param a the minuend
     * @param b the subtrahend
     * @return the difference
     */
    public static int subtract(int a, int b) {
        return a - b;
    }
    
    /**
     * Multiplies two numbers
     */
    public static int multiply(int a, int b) {
        return a * b;
    }
    
    /**
     * Divides a by b
     * @throws ArithmeticException if b is zero
     */
    public static double divide(int a, int b) {
        if (b == 0) {
            throw new ArithmeticException("Division by zero");
        }
        return (double) a / b;
    }
    
    /**
     * Applies a custom operation to two numbers
     */
    public static int applyOperation(int a, int b, BiFunction<Integer, Integer, Integer> operation) {
        int result = operation.apply(a, b);
        System.out.println("Result is: " + result);
        return result;
    }
    
    private static int privateHelper(int x) {
        return x >= 0 ? x : -x;
    }
}

/**
 * A calculator class that maintains state
 */
class CalculatorState {
    private int value;
    private List<String> history;
    
    /**
     * Creates a new calculator with value 0
     */
    public CalculatorState() {
        this.value = 0;
        this.history = new ArrayList<>();
    }
    
    /**
     * Creates a calculator with an initial value
     */
    public CalculatorState(int initialValue) {
        this.value = initialValue;
        this.history = new ArrayList<>();
    }
    
    /**
     * Gets the current value
     */
    public int getValue() {
        return value;
    }
    
    /**
     * Sets the current value
     */
    public void setValue(int value) {
        this.value = value;
    }
    
    /**
     * Adds a number to the current value
     */
    public int add(int n) {
        value += n;
        history.add("add " + n);
        return value;
    }
    
    /**
     * Subtracts a number from the current value
     */
    public int subtract(int n) {
        value -= n;
        history.add("subtract " + n);
        return value;
    }
    
    /**
     * Resets the calculator to zero
     */
    public void reset() {
        value = 0;
        history.clear();
    }
    
    /**
     * Gets the operation history
     */
    public List<String> getHistory() {
        return new ArrayList<>(history);
    }
    
    /**
     * Checks if calculator has any history
     */
    public boolean hasHistory() {
        return !history.isEmpty();
    }
    
    @Override
    public String toString() {
        return "Calculator(value: " + value + ")";
    }
    
    private void logOperation(String operation) {
        history.add(operation);
    }
}
