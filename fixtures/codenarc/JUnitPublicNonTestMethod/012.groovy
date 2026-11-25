
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }
                @BeforeAll void cleanUp() { println 'done' }
            }
        