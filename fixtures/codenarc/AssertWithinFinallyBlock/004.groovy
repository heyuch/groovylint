
            class MyClass {
                def myMethod(int count) {
                    try {
                        assert count > 0
                    }
                    finally {
                        println count
                    }
                }
            }