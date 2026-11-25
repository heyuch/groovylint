
            class MyClass {
                int myClosure = {
                    try {
                        doSomething()
                        return 0                // ok
                    } finally {
                        try {
                            // clean up
                            return 88           // BAD
                        } finally {
                            println "ok"
                        }
                        println 'finally'
                        return 99 }             // BAD
                    0                    // ok
                }
            }
        