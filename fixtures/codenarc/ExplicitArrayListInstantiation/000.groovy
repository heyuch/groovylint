
            def x = []
            class MyClass {
                def x = []
                def m(foo = []) {
                    def x = []
                    def y = new ArrayList() {   // anony inner class OK
                    }
                    def a1 = new ArrayList(x)    // constructor with parameter is OK
                    def a2 = new ArrayList(23)
                    def a3 = ArrayList([1, 2, 3])
                }
            }
        