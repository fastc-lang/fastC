package main

import "os"

func main() {
	var total int64
	for i := int64(1); i <= 1_000_000; i++ {
		total += i
	}
	os.Exit(int(total & 255))
}
