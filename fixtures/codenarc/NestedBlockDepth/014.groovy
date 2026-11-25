
            def myMethod() {
                while (notReady()) {          // 1
                    sleep(1000L)
                    myList.each { element ->  // 2
                        doSomething(element)
                    }
                }
            }
            