class M5 {
  void f() {
    var p =
        switch (x) {
          case String s when s.length() > 3 -> s;
          default -> "";
        };
  }
}
