
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }
                @BeforeEach void cleanUp() { println 'done' }
            }
        