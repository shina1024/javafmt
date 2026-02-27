class R11 {
  void f() {
    if (a) b();
    else if (c) d();
    else e();
  }

  boolean a, c;

  void b() {}

  void d() {}

  void e() {}
}
