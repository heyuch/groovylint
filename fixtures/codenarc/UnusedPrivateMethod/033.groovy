
            class MyClass {
                @ParameterizedTest
                @MethodSource('myTestArgs')
                void myTest(String arg1, String arg2) {
                    // ...
                }
                
                //@SuppressWarnings('UnusedPrivateMethod')
                private static Stream<Arguments> myTestArgs() {
                    Stream.of(Arguments.of('one', 'two'), Arguments.of('three', 'four'))
                }
            }
        