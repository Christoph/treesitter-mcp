namespace CalculatorApp.Models
{
    using System;

    /// <summary>
    /// Represents a point in 2D space
    /// </summary>
    public class Point
    {
        /// <summary>
        /// Gets or sets the X coordinate
        /// </summary>
        public double X { get; set; }

        /// <summary>
        /// Gets or sets the Y coordinate
        /// </summary>
        public double Y { get; set; }

        /// <summary>
        /// Creates a new point at the origin
        /// </summary>
        public Point()
        {
            X = 0;
            Y = 0;
        }

        /// <summary>
        /// Creates a new point with specified coordinates
        /// </summary>
        public Point(double x, double y)
        {
            X = x;
            Y = y;
        }

        /// <summary>
        /// Creates a point at the origin
        /// </summary>
        public static Point Origin()
        {
            return new Point(0, 0);
        }

        /// <summary>
        /// Calculates distance from the origin
        /// </summary>
        public double DistanceFromOrigin()
        {
            return Math.Sqrt(X * X + Y * Y);
        }

        /// <summary>
        /// Calculates distance to another point
        /// </summary>
        public double DistanceTo(Point other)
        {
            double dx = X - other.X;
            double dy = Y - other.Y;
            return Math.Sqrt(dx * dx + dy * dy);
        }

        /// <summary>
        /// Translates the point by the given offset
        /// </summary>
        public void Translate(double dx, double dy)
        {
            X += dx;
            Y += dy;
        }

        /// <summary>
        /// Returns a new point translated by the given offset
        /// </summary>
        public Point Translated(double dx, double dy)
        {
            return new Point(X + dx, Y + dy);
        }
    }

    /// <summary>
    /// Represents a line segment between two points
    /// </summary>
    public class LineSegment
    {
        /// <summary>
        /// Gets the starting point
        /// </summary>
        public Point Start { get; private set; }

        /// <summary>
        /// Gets the ending point
        /// </summary>
        public Point End { get; private set; }

        /// <summary>
        /// Creates a new line segment
        /// </summary>
        public LineSegment(Point start, Point end)
        {
            Start = start;
            End = end;
        }

        /// <summary>
        /// Calculates the length of the line segment
        /// </summary>
        public double Length()
        {
            return Start.DistanceTo(End);
        }

        /// <summary>
        /// Calculates the midpoint of the line segment
        /// </summary>
        public Point Midpoint()
        {
            double midX = (Start.X + End.X) / 2;
            double midY = (Start.Y + End.Y) / 2;
            return new Point(midX, midY);
        }
    }

    /// <summary>
    /// Interface for geometric shapes
    /// </summary>
    public interface IShape
    {
        /// <summary>
        /// Calculates the area of the shape
        /// </summary>
        double Area();

        /// <summary>
        /// Calculates the perimeter of the shape
        /// </summary>
        double Perimeter();

        /// <summary>
        /// Gets the center point of the shape
        /// </summary>
        Point Center { get; }
    }

    /// <summary>
    /// Represents a circle
    /// </summary>
    public class Circle : IShape
    {
        private Point _center;
        private double _radius;

        /// <summary>
        /// Gets the center of the circle
        /// </summary>
        public Point Center => _center;

        /// <summary>
        /// Gets or sets the radius
        /// </summary>
        public double Radius
        {
            get => _radius;
            set
            {
                if (value < 0)
                    throw new ArgumentException("Radius must be non-negative");
                _radius = value;
            }
        }

        /// <summary>
        /// Creates a new circle
        /// </summary>
        public Circle(Point center, double radius)
        {
            _center = center;
            Radius = radius;
        }

        /// <summary>
        /// Calculates the area of the circle
        /// </summary>
        public double Area()
        {
            return Math.PI * _radius * _radius;
        }

        /// <summary>
        /// Calculates the perimeter (circumference) of the circle
        /// </summary>
        public double Perimeter()
        {
            return 2 * Math.PI * _radius;
        }
    }
}
