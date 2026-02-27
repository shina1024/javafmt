class M11 {
  void f() {
    if (a) {
      if (b) {
        x();
      } else {
        y();
      }
    } else {
      z();
    }
  }

  boolean a, b;

  void x() {}

  void y() {}

  void z() {}
}
