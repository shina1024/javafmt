module m.probe {
  requires transitive java.base;
  requires static java.sql;

  exports p.api;
}
