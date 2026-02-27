class M2 {
  void f() {
    foo(bar(baz(qux(1, 2, 3), 4), 5), 6);
  }

  void foo(int x) {}

  int bar(int x, int y) {
    return x + y;
  }

  int baz(int x, int y) {
    return x + y;
  }

  int qux(int a, int b, int c) {
    return a + b + c;
  }
}
