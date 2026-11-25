
            class MyClass {
                private static def methodWithParameters(a,b) { a >> b }

                def x = {
                    methodWithParameters(10,1)
                }
            } 