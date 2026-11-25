
            def x = [:]
            class MyClass {
                def x = [:]
                def m(foo = [:]) {
                    def x = [:]
                    def y = new LinkedHashMap() {   // anonymous inner class OK
                    }
                    def m1 = new LinkedHashMap(x)   // constructor with parameter is OK
                    def m2 = new LinkedHashMap(23)
                    def m3 = new LinkedHashMap([a:1, b:2])
                }
            }
        