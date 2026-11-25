
            class MyClass {
                def myClosure = {
                    if (x==23) println '23'
                    println 'ok'
                    if (alreadyInitialized())
                        println 'initialized'
                }
            }
        