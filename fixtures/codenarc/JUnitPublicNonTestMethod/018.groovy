
            class MyTest extends GroovyTestCase {
                void testSomething() {
                    assert count > 0
                }
                protected boolean isReady() { true }
                private int calculate() { 23 }
            }
        