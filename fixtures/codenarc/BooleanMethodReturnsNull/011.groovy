
            class MyClass {
                void method() {
                    if (foo()) {
                        return
                    }
                    def closure = {
                        return false
                    }
                    closure.call()
                }
            }
        