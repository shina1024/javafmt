class P2 {
  boolean f(Object o) {
    return !(o instanceof String s) || s.isBlank();
  }
}
