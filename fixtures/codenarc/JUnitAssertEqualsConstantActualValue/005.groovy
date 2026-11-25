
            import org.junit.jupiter.api.Test
            import static org.junit.Assert.assertEquals

            class SampleTest {
                @Test
                def shouldFoo() {
                    //when
                    int result = 2

                    //then
                    assertEquals(result, 2)
                    assertEquals("Message", result, 2)
                    assertEquals(result, 2.3d, 0.5d)
                    assertEquals("Message", result, 2.3d, 0.5d)
                }
            }
        