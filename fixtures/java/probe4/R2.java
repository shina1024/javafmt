class R2 {
  void f() {
    java.util.function.Function<String, Integer> p = Integer::parseInt;
    var q = this::h;
  }

  int h(String s) {
    return s.length();
  }
}
