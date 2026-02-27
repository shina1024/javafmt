class P3 {
  void f(int x) {
    switch (x) {
      case 1 -> System.out.println(1);
      default -> {
        System.out.println(0);
      }
    }
  }
}
