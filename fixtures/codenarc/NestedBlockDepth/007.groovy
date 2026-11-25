
            def myMethod() {
                while (notReady()) {        // 1
                    sleep(1000L)
                    while(hasChars()) {     // 2
                    }
                }
            }
            