
            class MyClass {
                def foo() {}
                def foO() {}

                def x = new Object() {
                    def innerFoo() {}
                    def innerfoO = {} // this one is a closure!
                }
            }
        