class R8 {
  void f() {
    for (int i = 0, j = 1; i < 10 && j < 10; i++, j += 2) {
      x(i, j);
    }
  }

  void x(int a, int b) {}
}
