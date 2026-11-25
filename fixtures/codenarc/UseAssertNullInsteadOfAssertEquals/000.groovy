
             class MyTestCase extends TestCase {
    @Test
                void testMethod() {
                    assertEquals(1, foo())
                    assertTrue(foo())
                    assertTrue('message', foo())
                    assertSame(foo(), foo())
                    assertTrue(foo() > bar())
                }
              }
        