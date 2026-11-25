
            class MyClass {
                private int count = 0
                private other = 'abc'
            }
            class MyOtherClass {
                int defaultCount = count
                int other = 123
                void init() { other = 456 }
            }
        