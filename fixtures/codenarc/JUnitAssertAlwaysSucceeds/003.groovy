
            class MyTest extends TestCase {
                void testSomething() {
                    assertTrue(99)          // Not violations
                    assertTrue('error message', 'abc')
                    assertTrue([123])
                    assertTrue('error message', [a:123])

                    assertTrue(0)           // Not violations
                    assertTrue('error message', '')
                    assertTrue([])
                    assertTrue('error message', [:])
                }
            }
        