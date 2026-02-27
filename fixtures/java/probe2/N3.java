class N3 {
  int f(int x) {
    return switch (x) {
      case 1 -> 1;
      case 2, 3 -> 2;
      default -> 0;
    };
  }
}
