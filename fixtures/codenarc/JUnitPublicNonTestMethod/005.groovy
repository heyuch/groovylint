
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }

                void doSomething() {
                    // this does not need to be public
                }

                void setUp() {
                    super.setUp()
                }
                void tearDown() {
                    super.tearDown()
                }
            }
        