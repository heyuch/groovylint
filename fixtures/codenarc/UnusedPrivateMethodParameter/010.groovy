
            class MyClass {
                private myMethod1(Closure closure, def closure2, closure3) {
                    def value1 = closure()
                    def value2 = closure2.call()
                    return closure3(value1, value2)
                }
            }
        