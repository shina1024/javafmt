class H {
  void f() {
    var x = java.util.stream.Stream.of(1, 2, 3).map(i -> i + 1).filter(i -> i > 2).toList();
  }
}
