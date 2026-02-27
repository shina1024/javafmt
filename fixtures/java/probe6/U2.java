class U2 {
  @A(
      values = {1, 2, 3},
      name = "x")
  void f() {}

  @interface A {
    int[] values();

    String name();
  }
}
