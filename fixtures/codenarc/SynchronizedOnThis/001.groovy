
            class MyClass {
                private final def lock = new Object()

                def Method1() {
                    synchronized(lock) { ; }
                }
                def Method2() {
                    synchronized(lock) { ; }
                }
            }