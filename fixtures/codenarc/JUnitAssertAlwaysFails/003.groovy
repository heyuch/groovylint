
            class MyTest extends TestCase {
                void testSomething() {
                    assertTrue(0)
                    assertTrue('error message', '')
                    assertTrue([])
                    assertTrue('error message', [:])

                    assertTrue(99)          // Not violations
                    assertTrue('abc')
                    assertTrue([123])
                    assertTrue([a:123])
                }
            }
        