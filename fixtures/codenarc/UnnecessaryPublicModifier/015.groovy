
                class FakeTest {
                    @ParameterizedTest
                    @ValueSource(strings = [
                            'some public value',
                            'some other value',
                    ])
                    void testNothing(final String value) {
                        assertFalse(value.isEmpty())
                    }
                }
            