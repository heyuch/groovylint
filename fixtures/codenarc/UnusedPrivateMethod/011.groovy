
            class MyClass {
                private static def methodWithParameters(a,b) { a >> b }

                class MyInnerClass {
                    def call() {
                        methodWithParameters(10,1)
                    }
                }
            } 