class T3 {
  sealed interface S permits A, B {}

  record A(int x) implements S {}

  record B(String s) implements S {}

  int f(S s) {
    return switch (s) {
      case A a -> a.x();
      case B b -> b.s().length();
    };
  }
}
