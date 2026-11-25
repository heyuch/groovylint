
            class MyTest extends GroovyTestCase {
                public void testMethod() {
                    assertFalse(condition)
                    assertTrue(condition)
                    assertTrue(!condition, condition)
                    assertFalse(!condition, condition)
                }
            }
        