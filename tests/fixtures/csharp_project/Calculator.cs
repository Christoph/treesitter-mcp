/// <summary>
/// Calculator module with basic arithmetic operations
/// </summary>
namespace CalculatorApp
{
    using System;
    using CalculatorApp.Models;

    /// <summary>
    /// Provides basic arithmetic operations
    /// </summary>
    public static class Calculator
    {
        /// <summary>
        /// Adds two numbers together
        /// </summary>
        /// <param name="a">First number</param>
        /// <param name="b">Second number</param>
        /// <returns>The sum of a and b</returns>
        public static int Add(int a, int b)
        {
            return a + b;
        }

        /// <summary>
        /// Subtracts b from a
        /// </summary>
        /// <param name="a">The minuend</param>
        /// <param name="b">The subtrahend</param>
        /// <returns>The difference a - b</returns>
        public static int Subtract(int a, int b)
        {
            return a - b;
        }

        /// <summary>
        /// Multiplies two numbers
        /// </summary>
        public static int Multiply(int a, int b)
        {
            return a * b;
        }

        /// <summary>
        /// Divides a by b, returns null if b is zero
        /// </summary>
        public static int? Divide(int a, int b)
        {
            if (b == 0)
                return null;
            return a / b;
        }

        /// <summary>
        /// Applies a custom operation to two numbers
        /// </summary>
        public static int ApplyOperation(int a, int b, Func<int, int, int> operation)
        {
            int result = operation(a, b);
            Func<int, string> formatter = x => $"Result is: {x}";
            Console.WriteLine(formatter(result));
            return result;
        }

        /// <summary>
        /// Creates a new calculator instance
        /// </summary>
        public static CalculatorState CreateCalculator()
        {
            return new CalculatorState();
        }

        /// <summary>
        /// Creates a calculator with an initial value
        /// </summary>
        public static CalculatorState CreateCalculatorWithValue(int value)
        {
            return new CalculatorState(value);
        }

        /// <summary>
        /// Calculates the distance between two points
        /// </summary>
        public static double PointDistance(Point p1, Point p2)
        {
            double dx = p1.X - p2.X;
            double dy = p1.Y - p2.Y;
            return Math.Sqrt(dx * dx + dy * dy);
        }

        private static int PrivateHelper(int x)
        {
            return x >= 0 ? x : -x;
        }
    }

    /// <summary>
    /// A simple calculator class that maintains state
    /// </summary>
    public class CalculatorState
    {
        private List<string> _history;

        /// <summary>
        /// Gets or sets the current value
        /// </summary>
        public int Value { get; set; }

        /// <summary>
        /// Gets whether the calculator has any history
        /// </summary>
        public bool HasHistory => _history.Count > 0;

        /// <summary>
        /// Creates a new calculator with value 0
        /// </summary>
        public CalculatorState()
        {
            Value = 0;
            _history = new List<string>();
        }

        /// <summary>
        /// Creates a new calculator with the specified initial value
        /// </summary>
        public CalculatorState(int initialValue)
        {
            Value = initialValue;
            _history = new List<string>();
        }

        /// <summary>
        /// Adds a number to the current value
        /// </summary>
        public int Add(int n)
        {
            Value += n;
            _history.Add($"add {n}");
            return Value;
        }

        /// <summary>
        /// Subtracts a number from the current value
        /// </summary>
        public int Subtract(int n)
        {
            Value -= n;
            _history.Add($"subtract {n}");
            return Value;
        }

        /// <summary>
        /// Resets the calculator to zero
        /// </summary>
        public void Reset()
        {
            Value = 0;
            _history.Clear();
        }

        /// <summary>
        /// Gets the operation history
        /// </summary>
        public List<string> GetHistory()
        {
            return new List<string>(_history);
        }

        private void LogOperation(string operation)
        {
            _history.Add(operation);
        }

        public override string ToString()
        {
            return $"Calculator(value: {Value})";
        }
    }
}
