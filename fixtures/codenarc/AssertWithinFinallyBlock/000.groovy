
            class MyClass {
                boolean ready = true
                int myMethod() {
                    try {
                        doSomething()
                    } finally {
                        assert ready   // BAD
                    }
                }
            }
        