package types

type Point struct {
    X int
    Y int
}

type Calculator interface {
    Add(a int, b int) int
    Subtract(a int, b int) int
}
