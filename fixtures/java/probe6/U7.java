module m.u7 {
  requires transitive java.base;
  requires static java.sql;

  exports p.api;

  opens p.impl to
      x.y,
      z.w;

  uses p.spi.S;

  provides p.spi.S with
      p.impl.S;
}
