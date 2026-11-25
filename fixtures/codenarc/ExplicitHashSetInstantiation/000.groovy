
            def x = [] as Set
            class MyClass {
                def x = [] as Set
                def m(foo = [] as Set) {
                    def x = [] as Set
                    def y = new HashSet() {   // anony inner class OK
                    }
                    def s1 = new HashSet(x)   // constructor with parameter is OK
                    def s2 = new HashSet(23)
                    def s3 = new HashSet([a:1, b:2])
                }
            }
        