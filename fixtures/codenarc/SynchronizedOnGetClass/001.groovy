
                class MyClass {

                    def Method1() {
                        synchronized(getFoo()) { ; }
                    }
                    def Method2() {
                        synchronized(MyClass) { ; }
                    }
                    def Method3() {
                        synchronized(String.class) { ; }
                    }
                }