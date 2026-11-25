
            def x = [:]
            class MyClass {
                def x = [:]
                def m(foo = [:]) {
                    def x = [:]
                    def y = new HashMap() {   // anony inner class OK
                    }
                    def m1 = new HashMap(x)    // constructor with parameter is OK
                    def m2 = new HashMap(23)
                    def m3 = new HashMap([a:1, b:2])
                }
            }
        