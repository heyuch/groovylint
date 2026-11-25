
            def myClosure = () -> println "bla"
            myClosure = () -> { println "bla" }
            myClosure = () -> { }
            myClosure = () -> {
              println "bla"
            }
            myClosure = (def param1) -> { println "bla" }
            myClosure = param1 -> { println "bla" }
            myClosure = param1 -> println "bla"
            myClosure = (def param1, def param2) -> { println "bla" }
        