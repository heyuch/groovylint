
            class MyClass {
                def myMethod() {
                    try {                           // 1
                        while (notReady()) {        // 2
                            sleep(1000L)
                        }
                    }
                    catch(Exception e) {
                    }
                    finally {                       // 1
                        if (ready) {                // 2
                            println 'ready'
                        }
                    }
                }
            }
            