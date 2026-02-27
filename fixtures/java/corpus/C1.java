class C {
  int f(int n) {
    for (int i = 0; i < n; i++) {
      if (i % 2 == 0) {
        continue;
      } else {
        break;
      }
    }
    return n;
  }
}
