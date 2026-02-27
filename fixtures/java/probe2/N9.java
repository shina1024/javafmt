class N9 {
  void f() {
    this.<String>m("x");
  }

  <T> void m(T t) {}
}
