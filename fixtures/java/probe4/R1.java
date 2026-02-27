class R1 {
  void f() {
    try (var in1 = open();
        var in2 = open2()) {
      use(in1, in2);
    } catch (java.io.IOException | RuntimeException e) {
      throw new IllegalStateException(e);
    }
  }

  java.io.InputStream open() {
    return null;
  }

  java.io.InputStream open2() {
    return null;
  }

  void use(java.io.InputStream a, java.io.InputStream b) {}
}
