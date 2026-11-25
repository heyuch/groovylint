
            class MyTest extends GroovyTestCase {

                private static void assertSomething() {
                    testBar() // violation, even if in helper method
                    this.testBar() // violation, even if in helper method
                }
            }
        