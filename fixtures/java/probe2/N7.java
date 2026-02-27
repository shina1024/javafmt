class N7 {
  void f() {
    if (a && b || c) {
      x();
    }
  }

  boolean a, b, c;

  void x() {}
}
