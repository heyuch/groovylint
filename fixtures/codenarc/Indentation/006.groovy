
@SuppressWarnings          // Annotation: correct
 class MyClass { }         // Class: incorect --> IGNORED

  @Component              // Annotation: incorrect --> VIOLATION
class MyOtherClass { }    // Class: correct

@SuppressWarnings         // 1st Annotation: correct
  @Component              // 2nd Annotation: incorrect --> IGNORED
 class TestClass {        // Class: incorrect --> IGNORED

    @Provider                  // Annotation: correct
 private void doStuff() { }    // Method: incorrect --> IGNORED

    @Package void one() { }    // Method: correct
  @Package void two() { }      // Method: incorrect --> VIOLATION

    @SuppressWarnings          // Field: correct
       private String id       // Field: incorrect --> IGNORED

        @SuppressWarnings      // Field: incorrect --> VIOLATION
    private String name        // Field: correct
}
        