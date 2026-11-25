
            class MyTest {
                @Test
                void someMethod1() {
                    doSomething()
                    doSomethingElse()
                    // where is the assertion?
                }
                @org.junit.Test
                @Unknown
                @YetAnotherAnnotation // these annotations ALSO test the line number fix
                void someMethod2() {
                    doSomething()
                    doSomethingElse()
                    // where is the assertion?
                }
                
                @org.junit.jupiter.api.Test
                void someTestMethod3() {
                    println 123
                }
                
            }
        