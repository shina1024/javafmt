class N2 {
  void f() {
    do {
      x();
    } while (cond());
  }

  void x() {}

  boolean cond() {
    return true;
  }
}
