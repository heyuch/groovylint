
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }
                @AfterEach void cleanUp() { println 'done' }
            }
        