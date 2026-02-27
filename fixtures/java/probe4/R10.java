module m.example {
  requires java.base;

  exports a.b;

  opens a.c to
      x.y,
      z.w;

  uses a.spi.S;

  provides a.spi.S with
      a.impl.SImpl;
}
