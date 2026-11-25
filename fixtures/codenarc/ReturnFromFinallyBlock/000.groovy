
            class MyClass {
                int myMethod() {
                    if (debug) {
                        return 0                // ok
                    }
                    try {
                        doSomething()
                        return 0                // ok
                    } catch(Exception e) {
                        println 'exception'
                        return -1               // ok
                    } finally {
                        println 'finally'
                        return 99               // BAD
                    }
                }
            }
        