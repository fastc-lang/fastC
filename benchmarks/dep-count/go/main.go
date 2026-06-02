package main

import (
	"fmt"
	"net/http"
)

func main() {
	resp, err := http.Get("http://example.com")
	if err != nil {
		return
	}
	defer resp.Body.Close()
	fmt.Println("status:", resp.StatusCode)
}
