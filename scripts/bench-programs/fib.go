// Go 版 fibonacci(35) — L# ベンチマーク比較用
package main

import "fmt"

func fib(n int64) int64 {
	if n <= 1 {
		return n
	}
	return fib(n-1) + fib(n-2)
}

func main() {
	fmt.Println(fib(35))
}
