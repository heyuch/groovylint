
            def myMethod() {
                while (notReady()) {          // 1
                    sleep(1000L)
                    synchronized(this) {      // 2
                        doSomething()
                    }
                }
            }
            