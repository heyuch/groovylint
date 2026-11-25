
            class MyClass {
                def myMethod(int count) {
                    try {
                        doStuff(count)
                    }
                    catch(Exception e) {
                        assert count > 0
                    }
                    finally {
                        println count
                    }
                }
            }