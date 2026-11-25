
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }
                @Ignore void cleanUp() { println 'done' }
            }
        