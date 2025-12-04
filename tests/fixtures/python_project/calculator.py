"""Calculator module with basic arithmetic operations"""

from utils.helpers import validate_input, format_result


def add(a, b):
    """Adds two numbers together
    
    Args:
        a: First number
        b: Second number
        
    Returns:
        The sum of a and b
    """
    return a + b


def subtract(a, b):
    """Subtracts b from a"""
    return a - b


def multiply(a, b):
    """Multiplies two numbers"""
    return a * b


def divide(a, b):
    """Divides a by b, returns None if b is zero"""
    if b == 0:
        return None
    return a / b


def _private_helper(x):
    """Private helper function"""
    return x >= 0


def apply_operation(a, b, operation):
    """Applies a custom operation to two numbers
    
    Args:
        a: First number
        b: Second number
        operation: A function that takes two numbers
        
    Returns:
        Result of the operation
    """
    result = operation(a, b)
    
    # Nested function
    def formatter(x):
        return f"Result is: {x}"
    
    print(formatter(result))
    return result


class Calculator:
    """A simple calculator class
    
    This class maintains state for calculator operations.
    """
    
    def __init__(self, initial_value=0):
        """Creates a new Calculator
        
        Args:
            initial_value: Starting value (default: 0)
        """
        self.value = initial_value
        self._history = []
    
    def add(self, n):
        """Adds a number to the current value"""
        self.value += n
        self._history.append(f"add {n}")
        return self.value
    
    def subtract(self, n):
        """Subtracts a number from the current value"""
        self.value -= n
        self._history.append(f"subtract {n}")
        return self.value
    
    def reset(self):
        """Resets the calculator to zero"""
        self.value = 0
        self._history.clear()
    
    def get_history(self):
        """Gets the operation history"""
        return self._history.copy()
    
    def _log_operation(self, op):
        """Private helper method"""
        self._history.append(op)
    
    def __str__(self):
        return f"Calculator(value: {self.value})"


class Point:
    """A point structure for testing"""
    
    def __init__(self, x, y):
        """Creates a new point"""
        self.x = x
        self.y = y
    
    def distance_from_origin(self):
        """Calculates distance from origin"""
        return (self.x ** 2 + self.y ** 2) ** 0.5
