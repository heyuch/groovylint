class MyClass {
                def myMethod() {
                    try {
                        doSomething()
                    }
                    finally {
                        println "cleanup"
                    }
                }
            }