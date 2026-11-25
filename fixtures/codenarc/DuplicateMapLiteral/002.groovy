
            class MyClass {
                def init(String name) {
                    def var1 = [(name):1, b:1, c:1]
                }
                def cleanUp(String name) {
                    return [(name):1, b:1, c:1]
                }
            }
        