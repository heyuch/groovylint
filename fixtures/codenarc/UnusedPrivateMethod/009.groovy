
            class MyClass {
                private static def methodWithParameters(a,b) { a >> b }

                def myMethod() {
                    return new Class() {
                        def call() {
                            methodWithParameters(10,1)
                        }
                    }
                }
            } 