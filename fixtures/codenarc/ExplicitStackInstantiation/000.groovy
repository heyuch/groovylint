
            def x = [] as Stack
            class MyClass {
                def x = [] as Stack
                def m(foo = [] as Stack) {
                    def x = [] as Stack
                    def y = new Stack() {   // anony inner class OK
                    }
                    def m1 = new Stack(x)    // constructor with parameter is OK
                    def m2 = new Stack(23)
                    def m3 = new Stack([a:1, b:2])
                }
            }
        