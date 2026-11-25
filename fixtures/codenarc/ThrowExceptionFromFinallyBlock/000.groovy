
            class MyClass {
                int myMethod() {
                    if (debug) {
                        throw new Exception()   // ok
                    }
                    try {
                        doSomething()
                        throw new Exception()   // ok
                    } catch(Exception e) {
                        println 'exception'
                        throw new Exception()   // ok
                    } finally {
                        println 'finally'
                        throw new Exception()   // BAD
                    }
                }
            }
        