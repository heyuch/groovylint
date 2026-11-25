
             class MyTestCase extends TestCase {
                void testMethod() {
                    assert 1 == 2
                    assert 1 == 2 : 'message'

                    assertTrue()
                    foo.assertTrue(x)
                    foo.assertTrue('message', x)

                    assertFalse()
                    foo.assertFalse(x)
                    foo.assertFalse('message', x)

                    assertEquals()
                    assertEquals(x)
                    assertEquals(x, x, x, x)
                    foo.assertEquals('message', 1, 2)

                    assertNull()
                    assertNull(1, 2, 3)
                    foo.assertNull(x)
                    foo.assertNull('message', x)

                    assertNotNull()
                    assertNotNull(1, 2, 3)
                    foo.assertNotNull(x)
                    foo.assertNotNull('message', x)
                }
             }
        