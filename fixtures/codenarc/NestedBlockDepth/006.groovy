
            class MyClass {
                def myMethod() {
                    while(notReady()) {                 // 1
                        try {                           // 2
                            sleep(1000L)
                        }
                        catch(Exception e) {            // 2
                            log(e)
                        }
                    }
                }
            }
            