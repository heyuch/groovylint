
            class MyClass {
                boolean ready = false
                def myClosure = {
                    try {
                        doSomething()
                    } finally {
                        try {
                            assert ready    // BAD
                        } finally {
                            println "ok"
                        }
                        assert ready     // BAD
                    }
                }
            }
        