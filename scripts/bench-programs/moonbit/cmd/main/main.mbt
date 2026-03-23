///|
/// MoonBit 版 fibonacci(35) — L# ベンチマーク比較用
fn fib(n : Int) -> Int {
  if n <= 1 {
    n
  } else {
    fib(n - 1) + fib(n - 2)
  }
}

fn main {
  println(fib(35).to_string())
}
