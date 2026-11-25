
            class MyClass {
                static final GLOBAL_NAME = 'xxx'
                def doStuff() {
                    def otherVar
                    def defaultName = GLOBAL_NAME
                    String startName = defaultName
                    otherVar = startName.size()
                    println "name=$startName"
                    def amount = 123.45
                    println "amount=$amount"
                    return otherVar
                }
            }
        