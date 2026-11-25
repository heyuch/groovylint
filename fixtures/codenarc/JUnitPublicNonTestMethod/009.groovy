
            class MyTest extends GroovyTestCase {
                public void testSomething() { println 'ok' }
                @BeforeClass void initClass() { println 'ok' }
            }
        