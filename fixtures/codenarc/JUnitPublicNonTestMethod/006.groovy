
            class MyTest {
                @Test(expected = MyException.class)
                void shouldSendEmail() { assert count == 0 }
                
                @org.junit.Test
                void otherTest1() { assert count == 0 }
                
                @org.junit.jupiter.api.Test
                void otherTest2() { assert count == 0 }
            }
        