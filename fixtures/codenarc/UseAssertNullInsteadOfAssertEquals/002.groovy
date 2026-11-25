
             class MyTestCase extends TestCase {
                void testMethod() {
                    assertEquals('message', null, foo())
                    assertEquals('message', foo(), null)
                }
              }
        