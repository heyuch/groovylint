
            class MyClass {
                def myClosure = {
                    try {
                        doSomething()
                        throw new Exception()           // ok
                    } finally {
                        try {
                            // clean up
                            throw new Exception('A')    // BAD
                        } finally {
                            println "ok"
                        }
                        println 'finally'
                        throw new Exception('B') }     // BAD
                        throw new Exception()          // ok
                }
            }
        