class U10 {
  void f() {
    var x =
        switch (v) {
          case String s when s.length() > 2 -> s;
          default -> "";
        };
  }

  Object v;
}
