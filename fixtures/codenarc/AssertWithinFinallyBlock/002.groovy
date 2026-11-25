
            class MyClass {
                boolean ready = true
                def myMethod() {
                    try {
                        assert ready      // ok
                    }
                    finally {
                        println 'ok'
                    }
                    assert ready          // ok
                }
            }