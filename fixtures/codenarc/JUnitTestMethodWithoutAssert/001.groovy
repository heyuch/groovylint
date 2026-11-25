
            class MyTest {
                @Test(expected = IllegalArgumentException)
                void someTestMethod1() {
                    doSomething()
                }
                @Test(timeout = 1000)
                void someTestMethod2() {
                    doSomething()
                }
                @org.junit.Test(expected = IllegalArgumentException)
                void someTestMethod3() {
                    doSomething()
                }
                @org.junit.Test(timeout = 1000)
                void someTestMethod4() {
                    doSomething()
                }
            }
        