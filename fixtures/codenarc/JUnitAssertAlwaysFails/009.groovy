
            class MyTest extends TestCase {
                void testSomething() {
                    assertFalse(99)
                    assertFalse('error message', 'abc')
                    assertFalse([123])
                    assertFalse('error message', [a:123])

                    assertFalse(0)          // Not violations
                    assertFalse('')
                    assertFalse([])
                    assertFalse([:])
                }
            }
        