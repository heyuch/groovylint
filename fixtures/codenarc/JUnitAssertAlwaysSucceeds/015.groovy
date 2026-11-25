
            class MyTest extends TestCase {
                void testSomething() {
                    assertNotNull([])
                    assertNotNull([123])
                    assertNotNull([a:123])
                    assertNotNull([:])
                }
            }
        