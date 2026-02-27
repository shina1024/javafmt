class P4 {
  void f() {
    var r = java.util.List.of(1, 2, 3).stream().map(i -> i + 1).filter(i -> i > 2).toList();
  }
}
