
            class MyTest extends TestCase {
                void testSomething() {
                    assertNotNull(123)
                    assertNotNull('abc')
                    assertNotNull(true)
                    assertNotNull(false)
                    assertNotNull(Boolean.FALSE)
                    assertNotNull([])
                    assertNotNull([123])
                    assertNotNull([:])
                    assertNotNull([a:123])
                }
            }
        