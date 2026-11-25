
            class MyClass {
                def myClosure = {
                    if (x==23) { } else println '23'
                    println 'ok'
                    if (alreadyInitialized())
                        println 'initialized'
                    else
                        println 'not initialized'
                }
            }
        