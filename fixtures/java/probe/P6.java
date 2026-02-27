class P6 {
  sealed interface S permits A, B {}

  final class A implements S {}

  non-sealed class B implements S {}
}
