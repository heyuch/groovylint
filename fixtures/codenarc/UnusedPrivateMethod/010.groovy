
            class MyClass {
                private static def methodWithParameters(a,b) { a >> b }

                def myClosure = {
                    methodWithParameters(10,1)
                }
            } 