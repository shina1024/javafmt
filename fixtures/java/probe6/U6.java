class U6 {
  void f() {
    outer:
    for (int i = 0; i < 3; i++) {
      if (i == 2) break outer;
    }
  }
}
