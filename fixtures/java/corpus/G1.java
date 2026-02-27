class G {
  void f() {
    try {
      foo();
    } catch (Exception e) {
      bar();
    } finally {
      baz();
    }
  }

  void foo() {}

  void bar() {}

  void baz() {}
}
