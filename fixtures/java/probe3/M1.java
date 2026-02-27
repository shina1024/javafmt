class M1 {
  void f() {
    var x =
        java.util.stream.Stream.of(1, 2, 3, 4, 5)
            .map(i -> i + 1)
            .filter(i -> i % 2 == 0)
            .sorted()
            .toList();
  }
}
