
            class MyClass {
                def doSomething() {
                    def myClosure = { println 'ok' }
                    if (ready) {
                        myClosure()
                    }
                }
            }
        