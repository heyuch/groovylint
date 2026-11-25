
                package groovy.bugs

                class MyClass {
    @Test
                    void test () {
                        Foo.test()
                    }
                    class Foo {
                        private static test() { }
                    }
                }
        