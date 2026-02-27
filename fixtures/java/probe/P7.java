class P7 {
  void f() {
    try (var in = new java.io.ByteArrayInputStream(new byte[0])) {
      in.read();
    } catch (java.io.IOException e) {
      throw new RuntimeException(e);
    } finally {
      System.out.println("done");
    }
  }
}
