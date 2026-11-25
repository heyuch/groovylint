
             class MyTestCase extends TestCase {
    @Test
                void testMethod() {
                    assertEquals(1, foo())
                    assertTrue(foo())
                    assertTrue(x)
                    assertTrue('message', x)
                    assertTrue('message', foo())
                    assertSame(foo(), foo())
                    assertTrue(foo() > bar())
                    assertTrue(foo().is(bar(), baz()))
                    assertTrue(foo().is(bar()), x, y)
                }
              }
        