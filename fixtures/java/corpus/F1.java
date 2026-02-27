class F {
  void f() {
    Runnable r =
        () -> {
          System.out.println("x");
        };
    r.run();
  }
}
