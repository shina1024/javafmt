class R5 {
  enum E {
    A(1),
    B(2);
    final int n;

    E(int n) {
      this.n = n;
    }

    int n() {
      return n;
    }
  }
}
