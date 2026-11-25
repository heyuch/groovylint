
            class MyClass {
                def myMethod() {
                    synchronized(lock) {
                        println "bad stuff happened"
                    }
                }
            }