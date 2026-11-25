
            class MyTest {
                @Test
                void someTestMethod1() {
                    assert 1 == 2
                }
                @Test
                void someTestMethod2() {
                    assertEquals(1, 2)
                }
                @org.junit.Test
                void someTestMethod3() {
                    assert 1 == 2
                }
                @org.junit.Test
                void someTestMethod4() {
                    assertEquals(1, 2)
                }
                @org.junit.jupiter.api.Test
                void someTestMethod5() {
                    assertEquals(1, 2)
                }
            }
        