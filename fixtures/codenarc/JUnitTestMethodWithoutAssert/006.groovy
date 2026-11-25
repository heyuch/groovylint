
            class MyTest {
                @Rule
                public ExpectedException exception = ExpectedException.none()

                @Test
                void myTest() {
                    String nullString = null
                    exception.expect(NullPointerException)
                    nullString.toLowerCase()
                }

                @Test
                void myTest2() {
                    exception.expectMessage('argument')
                    throw new IllegalArgumentException('Illegal argument provided!')
                }
            }
        