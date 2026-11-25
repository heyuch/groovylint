
            class MyTest extends GroovyTestCase {
                boolean isThisNecessary() { false }
                void testSomething() {
                    assert count > 0
                }
                void doSomething() {
                    // this does not need to be public
                }
            }
        