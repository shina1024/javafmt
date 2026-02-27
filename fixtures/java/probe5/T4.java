class T4 {
  void f() {
    outer:
    for (int i = 0; i < 3; i++) {
      for (int j = 0; j < 3; j++) {
        if (i + j > 2) continue outer;
      }
    }
  }
}
