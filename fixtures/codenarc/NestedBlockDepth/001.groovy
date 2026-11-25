
            class MyClass {
                def myMethod() {
                    if (count > maxCount) {                 // 1
                        while(notReady()) {                 // 2
                            listeners.each { notify(it) }   // 3
                        }
                    }
                }
            }
            