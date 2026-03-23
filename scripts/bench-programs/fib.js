// JavaScript 版 fibonacci(35) — L# ベンチマーク比較用
function fib(n) {
  if (n <= 1) return n;
  return fib(n - 1) + fib(n - 2);
}

console.log(fib(35));
