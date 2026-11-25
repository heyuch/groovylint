
            class MyClass {
                def myMethod() {
                    a && b && c && d && e && f
                }
                def myClosure = { a ?: (b ?: c) }
                def otherClosure = { a ?: (b ?: c) }
                def myMethod2() {
                    a || b || c
                }
            }
        