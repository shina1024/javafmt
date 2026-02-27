class R6 {
  int f(int x) {
    return switch (x) {
      case 1 -> {
        yield g(1, 2, 3);
      }
      default -> {
        yield 0;
      }
    };
  }

  int g(int a, int b, int c) {
    return a + b + c;
  }
}
