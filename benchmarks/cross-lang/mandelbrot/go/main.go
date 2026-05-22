package main

import (
	"bufio"
	"os"
)

func main() {
	const width = 800
	const height = 800
	const maxIter = 100
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()

	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			cx := float64(x)/float64(width)*3.5 - 2.5
			cy := float64(y)/float64(height)*2.0 - 1.0
			var zx, zy float64
			i := 0
			for i < maxIter {
				zx2 := zx * zx
				zy2 := zy * zy
				if zx2+zy2 > 4.0 {
					break
				}
				newZx := zx2 - zy2 + cx
				newZy := 2.0*zx*zy + cy
				zx = newZx
				zy = newZy
				i++
			}
			if i == maxIter {
				out.WriteByte('*')
			} else {
				out.WriteByte(' ')
			}
		}
		out.WriteByte('\n')
	}
}
