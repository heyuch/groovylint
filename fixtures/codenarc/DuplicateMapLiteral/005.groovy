
            class MyClass {
                private count = 7
                def init(String name) {
                    def var1 = [a:name, b:1, c:1]

                    def var2 = [a:count+1, b:1, c:1]
                    def var3 = [a:count+1, b:1, c:1]
                }
                def cleanUp(String name) {
                    return [a:name, b:1, c:1]
                }
            }
        