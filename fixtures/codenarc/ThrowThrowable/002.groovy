class MyClass {
                def myMethod() {
                    try {
                        throw new Exception()      // ok
                    } finally {
                        println 'ok'
                    }
                    throw new Exception()          // ok
                }
            }