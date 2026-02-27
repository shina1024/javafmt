class T5 {
  void f() {
    do {
      a();
    } while (cond());
  }

  void a() {}

  boolean cond() {
    return true;
  }
}
