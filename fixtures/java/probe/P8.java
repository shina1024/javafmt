class P8 {
  void f() {
    Runnable r =
        () -> {
          System.out.println("x");
        };
    Runnable s = () -> System.out.println("y");
  }
}
