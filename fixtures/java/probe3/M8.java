class M8 {
  void f() {
    var r = this.<String>m("a").trim().toUpperCase().substring(0, 1);
  }

  String m(String s) {
    return s;
  }
}
