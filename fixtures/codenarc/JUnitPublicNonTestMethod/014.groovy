
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }
                @Disabled void cleanUp() { println 'done' }
            }
        