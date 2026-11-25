
            class MyClass {
                def myClosure = {                           // ignore
                    if (count > maxCount) {                 // 1
                        while(notReady()) {                 // 2
                            println 'not ready'
                        }
                    }
                }
            }
            