
            def x = [] as SortedSet
            class MyClass {
                def x = [] as SortedSet
                def m(foo = [] as SortedSet) {
                    def x = [] as SortedSet
                    def y = new TreeSet() {   // anony inner class OK
                    }
                    def m1 = new TreeSet(x)    // constructor with parameter is OK
                    def m2 = new TreeSet(23)
                    def m3 = new TreeSet([1,2,3])
                }
            }
        