
            class MyClass {
                def doSomething() {
                    def outerClosure = {
                        def innerClosure = { count -> println 'ok' }
                        innerClosure(99)
                    }
                    outerClosure()
                }
            }
        