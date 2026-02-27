class T2 {
  record P(int x, int y) {
    P {
      if (x < 0 || y < 0) {
        throw new IllegalArgumentException();
      }
    }
  }
}
