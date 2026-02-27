class R3 {
  void f() {
    assert cond() : "x";
    if (!cond()) {
      throw new AssertionError();
    }
  }

  boolean cond() {
    return true;
  }
}
