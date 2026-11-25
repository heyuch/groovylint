
             class MyTestCase extends TestCase {
                void testMethod() {
                    assertFalse(foo().is(bar()))
                    assertFalse("message", foo().is(bar()))
                }
              }
        