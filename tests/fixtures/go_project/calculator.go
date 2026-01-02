package calculator

import "fmt"

// Add adds two numbers together
func Add(a, b int) int {
    return a + b
}

// Subtract subtracts b from a
func Subtract(a, b int) int {
    return a - b
}

// Multiply multiplies two numbers
func Multiply(a, b int) int {
    return a * b
}

// Divide divides a by b
func Divide(a, b float64) float64 {
    if b == 0 {
        return 0
    }
    return a / b
}

// Calculator represents a calculator with history
type Calculator struct {
    History []string
}

// NewCalculator creates a new Calculator instance
func NewCalculator() *Calculator {
    return &Calculator{
        History: make([]string, 0),
    }
}

// AddToHistory adds an entry to the calculator history
func (c *Calculator) AddToHistory(entry string) {
    c.History = append(c.History, entry)
}

// GetHistory returns the calculator history
func (c *Calculator) GetHistory() []string {
    return c.History
}

// PrintHistory prints the calculator history
func (c *Calculator) PrintHistory() {
    for _, entry := range c.History {
        fmt.Println(entry)
    }
}
