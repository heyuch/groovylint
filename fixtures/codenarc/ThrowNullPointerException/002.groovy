class MyClass {
                def myMethod() {
                    try {
                        throw new MyException()      // ok
                    } finally {
                        println 'ok'
                    }
                    throw new OtherException()          // ok
                }
            }