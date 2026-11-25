
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }
                @AfterClass void cleanUp() { println 'done' }
            }
        