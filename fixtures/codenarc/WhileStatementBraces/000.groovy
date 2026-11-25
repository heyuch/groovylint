
            class MyClass {
                def myClosure = {
                    while (x==23) println '23'
                    println 'ok'
                    while (alreadyInitialized())
                        println 'initialized'
                }
            }
        