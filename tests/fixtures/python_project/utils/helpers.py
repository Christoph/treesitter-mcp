"""Helper utilities for the calculator"""


def validate_input(x):
    """Validates that input is non-negative
    
    Args:
        x: Number to validate
        
    Returns:
        True if valid, False otherwise
    """
    return x >= 0


def format_result(value):
    """Formats a result value
    
    Args:
        value: The value to format
        
    Returns:
        Formatted string
    """
    return f"Result: {value}"


def clamp(value, min_val, max_val):
    """Clamps a value between min and max
    
    Args:
        value: The value to clamp
        min_val: Minimum value
        max_val: Maximum value
        
    Returns:
        The clamped value
    """
    return max(min_val, min(value, max_val))


def apply_to_all(values, func):
    """Applies a function to all values in a list
    
    Args:
        values: List of values
        func: Function to apply
        
    Returns:
        List of results
    """
    return [func(v) for v in values]


def compose(f, g):
    """Composes two functions
    
    Args:
        f: First function
        g: Second function
        
    Returns:
        A new function that applies g then f
    """
    def composed(x):
        return f(g(x))
    return composed
