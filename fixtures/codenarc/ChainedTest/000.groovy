
            class MyTest extends GroovyTestCase {
                public void testFoo() {
                    // OK, no violation: one arg method is not actually a test method
                    5.times { testBar(it) }
                }

                private static void assertSomething() {
                    foo.testBar() // not a self/this call
                    Other.testBar() // not a self/this call
                }

                public void testBar() {
                    // ...
                }
            }
        