class M6 {
  void f() {
    try {
      a();
    } catch (Exception e) {
      throw new RuntimeException("x", e);
    } finally {
      b();
    }
  }

  void a() {}

  void b() {}
}
