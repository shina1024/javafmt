class E {
  void f() {
    switch (x) {
      case 1:
        foo();
        break;
      default:
        bar();
    }
  }

  int x;

  void foo() {}

  void bar() {}
}
