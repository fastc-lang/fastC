package main

import "os"

func fib(n int64) int64 {
	if n < 2 {
		return n
	}
	return fib(n-1) + fib(n-2)
}

func main() {
	r := fib(40)
	os.Exit(int(r & 255))
}
