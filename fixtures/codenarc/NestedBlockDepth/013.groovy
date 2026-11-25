
            def myMethod() {
                while (notReady()) {          // 1
                    sleep(1000L)
                    def closure = {           // 2
                        doSomething()
                    }
                }
            }
            