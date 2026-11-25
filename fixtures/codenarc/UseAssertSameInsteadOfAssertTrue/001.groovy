
             class MyTestCase extends TestCase {
                void testMethod() {
                    assertTrue(foo().is(bar()))
                    assertTrue("message", foo().is(bar()))
                }
              }
        