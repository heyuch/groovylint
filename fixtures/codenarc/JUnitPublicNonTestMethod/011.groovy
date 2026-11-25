
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }
                @AfterAll void cleanUp() { println 'done' }
            }
        