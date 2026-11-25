
            class MyTest extends TestCase {
                void testSomething() {
                    assertNull([])
                    assertNull([123])
                    assertNull([a:123])
                    assertNull([:])
                }
            }
        