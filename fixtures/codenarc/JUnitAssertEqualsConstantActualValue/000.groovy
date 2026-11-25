
            import org.junit.jupiter.api.Test
            import static org.junit.Assert.assertEquals

            class SampleTest {
                @Test
                def shouldFoo() {
                    //when
                    int result = 2

                    //then
                    assertEquals(2, result)
                    assertEquals("Message", 2, result)
                    assertEquals(2.3d, result, 0.5d)
                    assertEquals("Message", 2.3d, result, 0.5d)
                }
            }
        