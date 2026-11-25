
             class MyTestCase extends TestCase {
                @Test
                void testMethod() {
                    assertEquals(1, foo())
                    assertEquals('message', foo())
                    assertTrue(foo())
                    assertTrue('message', foo())
                    assertSame(foo(), foo())
                    assertTrue(foo() > bar())

                    assert 1 == foo()
                    assert 'message' == foo()
                    assert foo()
                    assert foo() : 'message'
                    assert foo().is(foo())
                    assert foo() > bar()
                }
              }
        