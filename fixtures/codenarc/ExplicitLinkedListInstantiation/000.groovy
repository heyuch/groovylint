
            def x = [] as Queue
            class MyClass {
                def x = [] as Queue
                def m(foo = [] as Queue) {
                    def x = [] as Queue
                    def y = new LinkedList() {   // anony inner class OK
                    }
                    def m1 = new LinkedList(x)    // constructor with parameter is OK
                    def m2 = new LinkedList(23)
                    def m3 = new LinkedList([1,2,3])
                }
            }
        