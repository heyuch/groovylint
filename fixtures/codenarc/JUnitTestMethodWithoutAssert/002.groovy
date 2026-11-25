
            class MyTest {
                private void testPrivate() {
                    // ignored because method is private
                }
                int testIntMethod() {
                    // ignored because method returns int
                }
                void testMethod1() {
                    assert 1 == 2
                }
                void testMethod2() {
                    assertEquals(1, 2)
                }
                void testMethod3() {
                    shouldFail {
                        foo()
                    }
                }
                void testMethod4() {
                    fail()
                }
                void testMethod5() {
                    verify()
                }
            }
        