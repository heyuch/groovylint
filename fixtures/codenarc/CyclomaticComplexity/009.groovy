
            class MyClass {
                def myMethod1() {
                    a && b && c
                }
                def myClosure = { a ?: (b ?: c) }

                def myMethod2() {
                    a || b || c
                }
            }
        