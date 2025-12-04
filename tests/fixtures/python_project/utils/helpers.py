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
    """Clamps a value between min and max"""
    return max(min_val, min(value, max_val))
