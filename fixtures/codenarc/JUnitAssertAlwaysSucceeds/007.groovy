
            class MyTest extends TestCase {
                void testSomething() {
                    assertFalse(0)
                    assertFalse('error message', '')
                    assertFalse([])
                    assertFalse('error message', [:])

                    assertFalse(99)          // Not violations
                    assertFalse('abc')
                    assertFalse([123])
                    assertFalse([a:123])
                }
            }
        