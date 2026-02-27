class U8 {
  record R<T extends Number>(T x, T y) {}

  R<Integer> r = new R<>(1, 2);
}
