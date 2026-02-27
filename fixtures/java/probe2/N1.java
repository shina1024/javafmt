class N1 {
  void f() {
    synchronized (this) {
      if (true) {
        a();
      } else {
        b();
      }
    }
  }

  void a() {}

  void b() {}
}
